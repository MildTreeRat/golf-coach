"""Build the golden vectors `scripts/conformance.py check` runs. [M19 P2]

Not a command — `conformance.py regenerate` is the entry point. This is the half that needs the
capture machine: it reads `data/processed/sessions/` for the real swings and
`tests/analysis/conftest.py` for the synthetic ones, neither of which a *checking* run should have
to possess. That split is the reason the vectors are committed at all.

**Two families, and they cover different things.**

*Synthetic* (`spec/vectors/synthetic/`) comes from `tests/analysis/conftest.py::make_swing`, which
is deterministic, RNG-free and pure stdlib — so it re-implements in another language exactly, and
a port can generate its own inputs rather than trusting ours. These are small, uncompressed and
readable, and they are where the *code paths* are covered: a window that has to be un-applied, a
checkpoint that fails, a checkpoint that cannot be scored at all, a bundle with no second view.

*Corpus* (`spec/vectors/corpus/`) is the fifteen real swings on disk, and covers what synthetic
input never can: real MediaPipe landmark noise, dropped-visibility frames, two genuinely
unsynchronised cameras and a launch monitor. These are where a port's *numerics* are judged.

**Corpus vectors ship the scored window and nothing else.** A stored clip is ~900-2000 frames of
33 landmarks and the whole set is 239 MB; sliced to the frames the pipeline actually scored it is
25.6 MB, and gzipped 8.3 MB. That is not a lossy reduction — `api/pipeline.py` picks the window
with `phases.select_swing` *before* the engine sees it, and frames outside it are never read by
anything that produces a number.

The consequence, which the vector records rather than hides: because the slice is handed over with
`face_on_window=None`, every frame index in `expected` is **window-relative**, where the stored
`analysis.json` beside the clip holds whole-clip indices. `provenance.frame_offset` is the
difference. `_verify_against_stored` below checks that adding it back reproduces the archive, which
is what makes these the corpus rather than fifteen plausible-looking files.

The landmarks are **not** reduced, though they could be: `analysis/` names 15 of the 33 and reads
`z` on none of them, which would take the set to 3.0 MB. Rejected because a reduced file is no
longer a `KeypointsFile` a port can parse with the shipped schema, and 5 MB is not worth a second
shape plus the test that would have to prove the reduction.

**Later families have sections of their own at the foot of this file**: the stages (M22 P1), the
format tables (M22 P3, M34 P2), the screen family (M34 P4), which is recorded once, by
`regenerate --screen-once`, and never rebuilt from here, and the storage family (M36), the corpus
reader and the stores, recorded once in the same way by `regenerate --storage-once`.
"""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib.util
import json
import math
import os
import random
import re
import struct
import sys
import tempfile
from collections.abc import Callable, Iterator
from difflib import SequenceMatcher
from pathlib import Path
from typing import Any

from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints, KeypointsFile
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.launch_monitor.screen.parser import _LINE_BUCKET
from golf_coach.launch_monitor.screen.profiles import load_profile, normalize_label

REPO = Path(__file__).resolve().parent.parent
VECTORS = REPO / "spec" / "vectors"
SESSIONS = REPO / "data" / "processed" / "sessions"

#: `make_swing` builds on a 100 fps clock (`conftest._FPS`), and the tempo checkpoint reads
#: timestamps rather than a frame rate, so this only has to be *consistent* with them — it is
#: metadata a port echoes, not a number anything divides by.
_SYNTHETIC_CLIP = ClipMetadata(fps=100.0, width=1080, height=1920)


def build_all() -> list[tuple[Path, dict[str, Any]]]:
    """Every vector this machine can build, as (path, payload) pairs for the caller to write."""
    out = [(VECTORS / "synthetic" / f"{name}.json", v) for name, v in _synthetic().items()]
    out += [(VECTORS / "corpus" / f"{name}.json.gz", v) for name, v in _corpus().items()]
    out += [(VECTORS / "audio" / f"{name}.json.gz", v) for name, v in _audio().items()]
    # The stages family, from the engine payloads **in hand** rather than from disk. Nothing has
    # been written yet at this point, so reading `spec/` here would record intermediates against
    # the previous generation's inputs — a stage vector derived from a vector that no longer
    # exists, and one that would pass every check because both halves are self-consistent.
    # `build_stages_from_disk` is the other entry, and it is right precisely because nothing is
    # being rewritten under it.
    stages = build_stages([v for _, v in out if v.get("provenance", {}).get("kind") != "audio"])
    out += stages
    # The format family, from the stage payloads in hand for the same reason — though here it
    # only borrows *inputs*, so a stale read would have been harmless rather than wrong. See
    # `build_format`.
    out += build_format([payload for _, payload in stages])
    return out


# --------------------------------------------------------------------------- synthetic


def _make_swing_module() -> Any:
    """Load `tests/analysis/conftest.py` as a plain module, by path.

    By path rather than by name because there are two `conftest.py` files on the way to it and
    neither `tests/` nor `tests/analysis/` is a package — pytest resolves that with its own
    importer and nothing else does.
    """
    return _load_conftest(REPO / "tests" / "analysis" / "conftest.py", "_conformance_conftest")


def _load_conftest(path: Path, name: str) -> Any:
    """Execute one `conftest.py` as a module of its own, under a name no other load uses."""
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader, f"cannot load {path}"
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _file(frames: list[FrameKeypoints]) -> KeypointsFile:
    return KeypointsFile(
        clip=_SYNTHETIC_CLIP.model_copy(update={"frame_count": len(frames)}),
        frames=frames,
        pose_estimator="synthetic:make_swing",
    )


def _synthetic() -> dict[str, dict[str, Any]]:
    """The cases chosen for the path they exercise, not for the swing they describe.

    Each row names why it is here. A case that only re-runs a path another case already covers is
    a vector that costs a regeneration and buys nothing.
    """
    make = _make_swing_module().make_swing
    cases: list[tuple[str, str, dict[str, Any]]] = [
        (
            "baseline-3to1",
            "30:10 tempo, steady head, held finish — every mechanics checkpoint passes. "
            "The case a port should get working first.",
            {"face_on": _file(make())},
        ),
        (
            "tempo-too-quick",
            "A 12:10 backswing:downswing — the tempo band is missed on the low side, so this "
            "is where a port's `passed` and its scored-versus-unscored split are first tested "
            "against a real verdict.",
            {"face_on": _file(make(backswing_frames=12, downswing_frames=10))},
        ),
        (
            "head-sway-and-finish-drift",
            "Head sway and finish drift together: two two-sided checkpoints missed at once, "
            "which is what separates a port that scores severity from one that scores sign.",
            {"face_on": _file(make(head_sway=0.09, finish_drift=0.08))},
        ),
        (
            "windowed",
            "The same baseline swing with 40 dead address frames welded on each end and a "
            "window naming the real one. Every frame index in `expected` is a whole-clip index, "
            "so this is the vector that fails a port which forgets to shift the window back "
            "(`engine._shifted`).",
            _windowed_case(make),
        ),
        (
            "no-handedness",
            "Handedness withheld. `head_stays_back` must land in `unscored` with reason "
            "`no_handedness` and must NOT be scored zero — ADR-010 §2 at its sharpest, and the "
            "one rule a port is most likely to break by accident.",
            {"face_on": _file(make()), "handedness": None},
        ),
        (
            "face-on-only",
            "No second view at all. `alignment` is None, a note says so, and the face-on result "
            "stands on its own — degradation reported rather than raised (ADR-013).",
            {"face_on": _file(make(backswing_frames=24, downswing_frames=9)), "handedness": "left"},
        ),
    ]

    built = {}
    for name, note, given in cases:
        payload = {"swing_id": "1", "session_id": "synthetic", "handedness": "right", **given}
        built[name] = _vector(f"synthetic/{name}", payload, kind="synthetic", note=note)
    return built


def _windowed_case(make: Any) -> dict[str, Any]:
    """A short swing buried in address frames, with the window that finds it.

    The padding is the swing's own first frame repeated, which is what a real clip's pre-address
    dwell looks like to the segmenter and keeps the padded region from containing a second
    detectable swing.
    """
    swing = make()
    pad = 40
    frames = [swing[0]] * pad + swing + [swing[-1]] * pad
    # `frame_index` and `timestamp_ms` are re-stamped across the whole padded clip, because a
    # repeated frame carrying a repeated timestamp would make the tempo arithmetic divide by a
    # zero-length backswing.
    ms = 1000.0 / (_SYNTHETIC_CLIP.fps or 100.0)
    stamped = [
        f.model_copy(update={"frame_index": i, "timestamp_ms": i * ms})
        for i, f in enumerate(frames)
    ]
    return {"face_on": _file(stamped), "face_on_window": [pad, pad + len(swing)]}


# --------------------------------------------------------------------------- corpus


def _corpus() -> dict[str, dict[str, Any]]:
    built = {}
    for analysis_path in sorted(SESSIONS.glob("*/*/analysis.json")):
        stored = json.loads(analysis_path.read_text(encoding="utf-8"))
        if stored.get("analysis_version") != ANALYSIS_VERSION:
            # A vector recorded off a stale artifact would pin the previous engine's answers.
            # `scripts/reanalyze.py` is what fixes this, and it must run first.
            print(
                f"  skip {analysis_path.parent.name}: stored at v{stored.get('analysis_version')},"
                f" engine is v{ANALYSIS_VERSION} — run scripts/reanalyze.py",
                file=sys.stderr,
            )
            continue
        name = f"{stored['session_id']}-{stored['swing_id']}"
        built[name] = _corpus_vector(analysis_path.parent, stored, name)
    return built


def _corpus_vector(swing_dir: Path, stored: dict[str, Any], name: str) -> dict[str, Any]:
    windows = {
        "face_on": stored.get("face_on_window"),
        "down_the_line": stored.get("down_the_line_window"),
    }
    handedness, loft = _identity(swing_dir)
    given: dict[str, Any] = {
        "swing_id": stored["swing_id"],
        "session_id": stored["session_id"],
        "shot": stored["swing"].get("shot"),
        "intent": stored["swing"].get("intent"),
        "handedness": handedness,
        "loft_deg": loft,
    }

    offsets: dict[str, int] = {}
    for role, window in windows.items():
        path = swing_dir / f"{role}.keypoints.json"
        if not path.exists():
            continue
        clip = KeypointsFile.model_validate_json(path.read_text(encoding="utf-8"))
        start, end = (window or [0, len(clip.frames)])[:2]
        offsets[role] = start
        frames = clip.frames[start:end]
        given[role] = KeypointsFile(
            # `frame_count` is what was decoded from the source, and this file no longer holds
            # that many frames. Restating it as the slice length keeps the envelope honest about
            # its own contents; `provenance` below is where the clip's real extent is recorded.
            clip=(clip.clip or ClipMetadata()).model_copy(update={"frame_count": len(frames)}),
            frames=frames,
            pose_estimator=clip.pose_estimator,
        )
        # Strikes are frame indices in the clip's own numbering, so they move with the slice.
        # A strike outside the window is dropped rather than clamped: clamping would invent a
        # transient at the window edge and pin the alignment to it.
        heard = _strikes(swing_dir, role)
        given[f"{role}_strikes"] = [s - start for s in heard if start <= s < end] or None

    vector = _vector(
        f"corpus/{name}",
        given,
        kind="corpus",
        note=f"real capture, {swing_dir.relative_to(REPO).as_posix()}",
        extra={"frame_offset": offsets, "source_clip_frames": _source_frames(swing_dir)},
    )
    _verify_against_stored(vector, stored, offsets, name)
    return vector


def _identity(swing_dir: Path) -> tuple[str | None, float | None]:
    """The handedness and the declared loft, resolved exactly the way `api/pipeline.py` does.

    These are the two arguments `analysis` is forbidden to fetch for itself (ADR-008: opening a
    bag file to find out what a `7i` is bent to is the shell's job), so a vector has to fetch them
    the same way the shell did or it is not the same run.

    **Read from the registry and the bag rather than recovered from the artifact**, which was the
    first thing tried and is wrong: a loft that the flight solve *refused* on leaves no
    `club_loft_deg` measurement behind, so recovering it from the result silently produced `None`
    for `2026-08-23/2` and a vector whose refusal sentence blamed a missing loft the run had. The
    price is that these read today's files rather than the day's, and `_verify_against_stored`
    below is what makes that safe: an edited bag fails the build instead of quietly re-pinning.
    """
    from golf_coach.config import settings
    from golf_coach.storage.flight_inputs import loft_for_club
    from golf_coach.storage.golfer_store import GolferStore
    from golf_coach.storage.manifest import SwingManifest

    manifest = SwingManifest.model_validate_json(
        (swing_dir / "manifest.json").read_text(encoding="utf-8")
    )
    loft, _gap = loft_for_club(manifest.player_id, manifest.club, golfers_dir=settings.golfers_dir)
    if manifest.player_id is None:
        return None, loft
    golfer = GolferStore(settings.golfers_dir).get(manifest.player_id)
    hand = golfer.handedness if golfer else None
    return (Handedness(hand).value if hand else None), loft


def _strikes(swing_dir: Path, role: str) -> list[int]:
    """Ball strikes heard in one view, as frame indices in that clip's own numbering.

    Reads the stored `{role}.audio.json` rather than re-running `audio/impact.py`: the roadmap's
    own note for this phase is that `detect_strikes` has no reproducible oracle in the suite,
    because `tests/audio/test_impact.py` synthesizes its clips from a seeded numpy RNG. The stored
    artifacts are the golden set, and they are what the pipeline read too.
    """
    path = swing_dir / f"{role}.audio.json"
    if not path.exists():
        return []
    audio = json.loads(path.read_text(encoding="utf-8"))
    # File order, not sorted, and not deduplicated — `api/pipeline.py:1165` builds the list this
    # way and `with_measured_impact` takes the nearest strike to the pose impact, so a reordering
    # here is a different tie-break there.
    return [s["frame"] for s in audio.get("strikes", []) if s.get("frame") is not None]


def _source_frames(swing_dir: Path) -> dict[str, int]:
    """How long each clip really was, so a reader can see what the slice left out."""
    out = {}
    for role in ("face_on", "down_the_line"):
        path = swing_dir / f"{role}.keypoints.json"
        if path.exists():
            out[role] = len(json.loads(path.read_text(encoding="utf-8")).get("frames", []))
    return out


def _verify_against_stored(
    vector: dict[str, Any], stored: dict[str, Any], offsets: dict[str, int], name: str
) -> None:
    """Assert the windowed vector reproduces the archived analysis, shifted by its own offset.

    Without this the corpus vectors are fifteen files that agree with whatever this build happens
    to produce — self-consistent and evidence of nothing. With it they are the swings on disk:
    the scored path receives byte-identically the same frames either way, so the scores and
    measurements must match exactly, and the phase boundaries must match once the offset is added
    back. A mismatch means the inputs were reconstructed wrongly and the vector is not the swing
    it claims to be.
    """
    from conformance import compare_results

    got, want = vector["expected"]["swing"], stored["swing"]
    for field in ("checkpoint_scores", "measurements", "unscored", "mechanics_score",
                  "overall_score", "outcome_score", "shot", "intent"):
        diffs = compare_results(want.get(field), got.get(field), f"swing.{field}")
        if diffs:
            raise AssertionError(
                f"{name}: the windowed vector does not reproduce the stored analysis at "
                f"{field} — {diffs[0]}"
            )

    # `feedback` hangs off the bundle rather than the swing, and it is the half a golfer reads.
    # It is also the half `analyze_swing_bundle` does not produce — `run_vector` adds it the way
    # `api/pipeline.py` does, and this is what proves the two agree.
    diffs = compare_results(stored.get("feedback"), vector["expected"].get("feedback"), "feedback")
    if diffs:
        raise AssertionError(f"{name}: the ranked feedback does not match the archive — {diffs[0]}")

    offset = offsets.get("face_on", 0)
    shifted = [
        {**p, "start_frame": p["start_frame"] + offset, "end_frame": p["end_frame"] + offset}
        for p in got.get("phases", [])
    ]
    diffs = compare_results(want.get("phases"), shifted, "swing.phases")
    if diffs:
        raise AssertionError(f"{name}: phases do not shift back onto the archive — {diffs[0]}")


# --------------------------------------------------------------------------- envelope


def _vector(
    vector_id: str,
    given: dict[str, Any],
    *,
    kind: str,
    note: str,
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Serialize the input, run it through `conformance.run_vector`, and pair the two.

    Through `run_vector` rather than calling `analyze_swing_bundle` here, because that function is
    the definition of what a port is asked to reproduce. A second call site with its own argument
    mapping would be a second definition, and the vectors would be built against whichever of the
    two was wrong — the expensive kind of correct-looking file.

    The input is serialized **before** it is run, so what gets recorded is what a reader will parse
    back rather than the live objects that built it. A field that does not survive the round trip
    fails here, at build time, instead of in a port six months from now.
    """
    from conformance import run_vector

    given_json = {k: _jsonable(v) for k, v in given.items()}
    return {
        "id": vector_id,
        "analysis_version": ANALYSIS_VERSION,
        "provenance": {"kind": kind, "note": note, **(extra or {})},
        "input": given_json,
        "expected": run_vector({"input": given_json}),
    }


def _jsonable(value: Any) -> Any:
    return json.loads(value.model_dump_json()) if hasattr(value, "model_dump_json") else value


# --------------------------------------------------------------------------- stages (M22 P1)


def build_stages(engine_vectors: list[dict[str, Any]]) -> list[tuple[Path, dict[str, Any]]]:
    """One stage vector per engine vector, as (path, payload) pairs for the caller to write.

    **Derived from the committed engine vectors, not from `data/processed/`.** That is the whole
    reason this family can be rebuilt on a machine with no captures on it, which the corpus family
    cannot (ADR-032 §2, `docs/CONFORMANCE.md` §4). The engine vector already carries its input
    serialized, so the input is referenced by id here and duplicated nowhere — the 8.6 MB stays in
    one place.

    Mirrors the engine family's layout and compression decision exactly: `stages/synthetic/*.json`
    uncompressed because being readable is most of a synthetic vector's value, and
    `stages/corpus/*.json.gz` because nobody will ever read a diff of 261 frames of float text.
    """
    from conformance import run_stages

    out = []
    for vector in engine_vectors:
        vector_id = vector["id"]
        stages = run_stages(vector)
        _verify_stages_compose(stages, vector)
        gz = vector_id.startswith("corpus/")
        path = VECTORS / "stages" / f"{vector_id}.json{'.gz' if gz else ''}"
        out.append(
            (
                path,
                {
                    "id": f"stages/{vector_id}",
                    "analysis_version": ANALYSIS_VERSION,
                    "provenance": {
                        "kind": "stages",
                        # The input, by reference. A reader resolves it to
                        # `spec/vectors/{derived_from}.json[.gz]` and reads `input` from there.
                        "derived_from": vector_id,
                        "note": (
                            f"intermediates of {vector_id}, recorded at each function boundary "
                            f"`analyze_swing_bundle` already has"
                        ),
                    },
                    "stages": stages,
                },
            )
        )
    return out


def build_stages_from_disk() -> list[tuple[Path, dict[str, Any]]]:
    """What `regenerate --stages-only` runs: read the committed engine vectors, rebuild stages.

    Separate from `build_all` because the full regeneration has the engine payloads *in hand* and
    must use those rather than what is on disk — it has just rewritten them, and a stage vector
    built from the previous generation's file would be recorded against an input that no longer
    exists. That is the same trap `_corpus` avoids by refusing a stale `analysis.json`.
    """
    from conformance import _read_json, engine_vector_paths

    return build_stages([_read_json(path) for path in engine_vector_paths()])


def _verify_stages_compose(stages: dict[str, Any], vector: dict[str, Any]) -> None:
    """Assert the recorded intermediates add back up to the vector's committed bundle answer.

    **Without this the stage family is twenty-one files that agree with whatever `run_stages`
    happens to do**, which is exactly the objection `_verify_against_stored` exists to answer for
    the corpus family. `run_stages` re-orchestrates the engine — it calls the engine's own
    functions, but the *order* it calls them in is a second copy of `analyze_swing_bundle`'s, and
    a second copy is a thing that drifts. This is what catches the drift: every stage that
    contributes to `expected` is composed forward and compared against it under the same
    `compare_results` a port is judged by.

    The two stages this cannot fully compose are named rather than skipped silently: `smoothed`
    reaches `expected` only through `phases` and `measure` (so the pass-through property is
    asserted directly instead), and `alignment`'s anchor pairs reach it only through
    `clip_alignment`, which *is* compared.
    """
    from conformance import _tuple, compare_results

    from golf_coach.analysis.engine import _shifted, _windowed
    from golf_coach.analysis.smoothing import smooth_keypoints
    from golf_coach.contracts.swing import PhaseSegment

    name = vector["id"]
    given, expected = vector["input"], vector["expected"]
    swing = expected["swing"]

    def fail(what: str, diffs: list[Any]) -> None:
        if diffs:
            raise AssertionError(
                f"{name}: the {what} stage does not compose onto `expected` — {diffs[0]}"
            )

    # `smoothed`: the four fields it does not record are asserted to be copies, which is what
    # makes not recording them honest rather than merely cheap. `camera_id` is *not* among them —
    # `smooth_keypoints` builds its `FrameKeypoints` without one, so the smoothed timeline drops
    # it. Nothing reads it back (`engine._camera_id` runs on the raw frames), but a port that
    # carried it through would differ from this reference on a field no vector would ever show.
    face_on = KeypointsFile.model_validate(given["face_on"])
    start, frames = _windowed(face_on.frames, _tuple(given.get("face_on_window")))
    smoothed = smooth_keypoints(frames)
    assert len(smoothed) == len(stages["smoothed"]) == len(frames), f"{name}: smoothed frame count"
    for raw, got in zip(frames, smoothed, strict=True):
        assert (got.frame_index, got.timestamp_ms) == (raw.frame_index, raw.timestamp_ms), (
            f"{name}: smoothing moved a frame index or timestamp, which it does not do"
        )
        assert got.camera_id is None, f"{name}: smoothing is expected to drop camera_id"
        for a, b in zip(raw.landmarks, got.landmarks, strict=True):
            assert (a.z, a.visibility) == (b.z, b.visibility), (
                f"{name}: smoothing moved a z or a visibility, which it does not do — "
                f"the `smoothed` stage records x/y only and that is no longer safe"
            )

    # `phases`: window-relative here, whole-clip in the artifact. Shifting them back is what
    # exercises `engine._shifted`, and `synthetic/windowed` is the only vector where it moves.
    shifted = [_shifted(PhaseSegment.model_validate(p), start) for p in stages["phases"]]
    fail("phases", compare_results(swing["phases"], [_jsonable(p) for p in shifted], "phases"))

    # `measure`: the unrounded float, rounded the way `_measurements` rounds it, must be the row
    # that reached the artifact — which puts Python's banker's rounding under test at build time
    # rather than leaving it for a port to discover.
    pose_rows = {r["name"]: r for group in stages["measurements"] if group["group"] == "pose"
                 for r in group["rows"]}
    for row in stages["measure"]:
        stored = pose_rows.get(row["name"])
        if row["value"] is None:
            assert stored is None, (
                f"{name}: {row['name']} refused in `measure` but reached `measurements`"
            )
            assert row["reason"], f"{name}: {row['name']} refused with no reason"
            continue
        assert stored is not None, f"{name}: {row['name']} measured but absent from `measurements`"
        fail(f"measure/{row['name']}", compare_results(
            stored["value"], round(row["value"], 4), f"measure.{row['name']}"
        ))

    # `measurements`: the groups are a partition of one flat list, so concatenating them in the
    # recorded order is the whole check.
    flat = [row for group in stages["measurements"] for row in group["rows"]]
    fail("measurements", compare_results(swing["measurements"], flat, "measurements"))

    # `checkpoints`: containment rather than equality, and deliberately. Two things happen to this
    # stage's answer after it: `flight_unscored` extends `unscored`, and a late top removes a
    # score under `cross_view_contradicted` (`engine._without_contradicted_scores`). Composing
    # those forward here would be a third copy of the engine; checking that every outcome landed
    # somewhere it is allowed to land tests the same thing without one.
    scored = {s["name"]: s for s in swing["checkpoint_scores"]}
    refused = {u["name"]: u for u in swing["unscored"]}
    for row in stages["checkpoints"]:
        if row["score"] is not None:
            if row["name"] in scored:
                fail(f"checkpoints/{row['name']}", compare_results(
                    scored[row["name"]], row["score"], f"checkpoints.{row['name']}"
                ))
            else:
                assert refused.get(row["name"], {}).get("reason") == "cross_view_contradicted", (
                    f"{name}: {row['name']} scored in `checkpoints` but is neither in "
                    f"`checkpoint_scores` nor withdrawn by a late top"
                )
            continue
        assert row["name"] in refused, f"{name}: {row['name']} refused but is not in `unscored`"
        fail(f"checkpoints/{row['name']}", compare_results(
            refused[row["name"]]["reason"], row["reason"], f"checkpoints.{row['name']}.reason"
        ))

    # `alignment`: the anchors are not in the artifact, but the warp built from them is.
    fail("alignment", compare_results(
        expected.get("alignment"), stages["alignment"]["clip_alignment"], "alignment"
    ))

    # `flight`: the refusals it produces are appended to `unscored` verbatim.
    for entry in stages["flight"]["unscored"]:
        assert entry["name"] in refused, (
            f"{name}: the flight refused {entry['name']} and the artifact does not say so"
        )
        fail(f"flight/{entry['name']}", compare_results(
            refused[entry["name"]], entry, f"flight.unscored.{entry['name']}"
        ))


# --------------------------------------------------------------------------- audio (M20)


def _audio() -> dict[str, dict[str, Any]]:
    """Nothing. The audio family was recorded once and is not regenerable from here.

    **This is a deliberate refusal, not a gap, and it is the price of M20's delete.**
    `spec/vectors/audio/` was built by `audio/impact.py` — the numpy detector this repo carried
    from M11 to M20 — and verified against the thirty `{role}.audio.json` the pipeline had already
    written. Then `crates/trigger` reproduced it, every sample index exact and every float to a
    worst relative difference of 7.7e-15, and `impact.py` was deleted (ADR-030's addendum: Python
    keeps only what does not translate).

    Which leaves no Python able to rebuild them — and rebuilding them from the Rust detector would
    be worse than not rebuilding them at all. An oracle recorded by the implementation it judges
    is a self-portrait: it would pass by construction and detect nothing, which is precisely the
    silent disagreement ADR-030 §8 exists to prevent. The committed files are the reference, and
    they outlive the code that produced them. That is the whole argument of M19.

    So `regenerate` leaves them alone — it writes over vectors rather than clearing the directory,
    so returning nothing here is enough. `cargo test` is what runs them.

    **If `AUDIO_DETECTOR_VERSION` ever moves**, re-recording is a decision rather than a command,
    and it needs a new oracle named before it: a `golf-trigger envelope` subcommand plus an
    argument for why the new answers are right, or a re-derivation from the stored artifacts.
    Leaving a working-looking rebuild path here would let that decision be skipped by running a
    script.
    """
    return {}


# --------------------------------------------------------------------------- format (M22 P3)
#
# ADR-032 §3's three portability edges, as a table of Python's exact answers. The implementation
# under test is `crates/pyfmt/src/lib.rs`, so `conformance.py check` defers these the way it
# defers the audio family — there is no second Python implementation to run them against, and
# running Python against itself would gate nothing.
#
# **This is the first vector family that does not age on `ANALYSIS_VERSION`.** It records the
# behaviour of `round()`, `format()` and `sorted()`, which belong to the language rather than to
# the engine, so a version bump leaves every answer here true. `python_version` is what it ages
# on, and that only matters if CPython changes a rounding rule — which it has not since 3.1 gave
# `repr` its shortest-round-trip guarantee and `round` its decimal-correct one.
#
# **Every float travels as `repr`, not as a JSON number, and that is load-bearing twice.** `repr`
# is shortest-round-trip, so `float(repr(x)) == x`, and Rust's correctly-rounded `str::parse`
# lands on the identical bits — the table can therefore be compared with `==` rather than with a
# tolerance, which is the whole point of a rounding gate. And `json.dumps(float("nan"))` writes a
# bare `NaN` that `serde_json` rejects outright, while `repr(float("nan"))` is `"nan"`, which
# Rust's parser accepts. Non-finite input is not decoration here: `.Nf` on a NaN is one of only
# two places these two languages actually print different text.


#: The `ndigits` `round(x, n)` is called with across the surface `run_vector` reaches — measured
#: with `grep -rno 'round([^()]*, *[0-9])' src/golf_coach/analysis/`, not chosen. Anything outside
#: this set has no caller in the engine and so cannot be tested here without inventing one.
_ROUND_NDIGITS = (1, 2, 4)

#: The precisions `:.Nf` is interpolated with, from the same kind of sweep over `analysis/` and
#: `feedback/`: 23 `.0f` sites, 9 `.1f`, 30 `.2f` and 13 `.3f`.
_FIXED_PRECISIONS = (0, 1, 2, 3)

#: `%g` in CPython's `float.__format__` with no precision given. Hard-coded because there is
#: nothing to derive it from — it is C's default, and every `:g` site in the engine relies on it.
_G_PRECISION = 6


def build_format(stage_vectors: list[dict[str, Any]] | None = None) -> list[
    tuple[Path, dict[str, Any]]
]:
    """The formatting edges' answer table, as (path, payload) pairs for the caller to write.

    Four edges rather than ADR-032 §3's three: `repr.json` is M22 P5's, an f-string with no format
    spec at all, and it is here rather than in a Rust unit test for the reason the other three are
    — the answer belongs to CPython and only CPython can record it.

    `stage_vectors` is where `_engine_floats` and `_format_sum` get their realistic inputs; `None`
    reads them from disk. **Unlike the stages family, a stale read here is harmless**, and the
    asymmetry is worth naming because the two functions look alike. A stage vector records an
    *answer* the engine gave, so building one against a superseded input produces a file that is
    self-consistent and wrong. This family takes only *inputs* from over there — every `expected`
    comes from CPython's `round`, `format` and `sum`, which do not know what an engine is — so a
    value that has since stopped being produced is still a float Python rounds exactly the same
    way.

    Uncompressed, unlike the other families of comparable size: these files are meant to be *read*
    when a Rust test fails, and a reader who has to decompress a vector to see that 0.145 rounds to
    0.14 will guess instead.

    The next five tables are the screen parser's (M34 P2), and the last three the many-shot
    layer's (M36 P4); each group has its own section below.
    """
    if stage_vectors is None:
        # Read once here rather than in each consumer, because two of them want it now and the
        # stages family is most of `spec/`'s bytes.
        from conformance import _read_json, stage_vector_paths

        stage_vectors = [_read_json(path) for path in stage_vector_paths()]
    pool = _engine_floats(stage_vectors)
    return [
        (VECTORS / "format" / "rounding.json", _format_rounding(pool)),
        (VECTORS / "format" / "fixed.json", _format_fixed(pool)),
        (VECTORS / "format" / "general.json", _format_general(pool)),
        (VECTORS / "format" / "repr.json", _format_repr(pool)),
        (VECTORS / "format" / "ordering.json", _format_ordering()),
        (VECTORS / "format" / "str_repr.json", _format_str_repr()),
        (VECTORS / "format" / "floor_div.json", _format_floor_div()),
        (VECTORS / "format" / "text_case.json", _format_text_case()),
        (VECTORS / "format" / "sum.json", _format_sum(stage_vectors)),
        (VECTORS / "format" / "difflib_ratio.json", _format_difflib_ratio()),
        (VECTORS / "format" / "general_precision.json", _format_general_precision(pool)),
        (VECTORS / "format" / "lower.json", _format_lower()),
        (VECTORS / "format" / "timestamp.json", _format_timestamp()),
    ]


#: Where the first five tables' edges are written down. The parser's tables pass their own.
_ENGINE_EDGES = "ADR-032 §3; docs/CONFORMANCE.md §3 'The known edges a Rust port will hit'"


def _format_vector(
    vector_id: str,
    note: str,
    cases: list[dict[str, Any]],
    *,
    edges: str = _ENGINE_EDGES,
    implemented_by: str | None = None,
) -> dict[str, Any]:
    assert cases, f"{vector_id} recorded no cases"
    provenance: dict[str, Any] = {"kind": "format", "note": note, "edges": edges}
    if implemented_by is not None:
        # Absent on the five engine tables, which predate the key and are `pyfmt`'s by its absence:
        # writing it into them would churn five files whose every answer is unchanged. See the
        # parser section's header.
        provenance["implemented_by"] = implemented_by
    return {
        "id": vector_id,
        # Deliberately not `analysis_version` — see this section's header. `check` keys its
        # staleness test on the field *name*, so carrying the engine's would make every version
        # bump report four stale vectors with nothing to regenerate about them.
        "python_version": _python_version(),
        "provenance": provenance,
        "cases": cases,
    }


def _python_version() -> str:
    """The interpreter that answered, as `3.13.3` — the format family ages on it, the screen
    family records it."""
    return ".".join(str(part) for part in sys.version_info[:3])


def _exact_ties(ndigits: int, count: int) -> list[float]:
    """Every value that lands *exactly* on a tie at `ndigits` decimal places, in order.

    **The tie cases are enumerable rather than sampled, which is what keeps this table small.** A
    tie needs `x * 10**n == j + 0.5` to hold with no representation error, so
    `x = (2j + 1) / (2 * 10**n)`, and a binary float can only be that if the `5**n` in the
    denominator cancels — which forces `2j + 1` to be a multiple of `5**n` and leaves
    `x = (2i + 1) / 2**(n + 1)`. The exact ties at `n` places are therefore the odd multiples of
    `2**-(n + 1)`, and nothing else is one.

    Which is also why a random sweep is worth nothing for this edge: half-to-even and
    half-away-from-zero differ *only* on these values, and a uniform draw hits none of them.
    """
    step = 2.0 ** -(ndigits + 1)
    return [step * (2 * i + 1) for i in range(count)]


def _tie_neighbourhood(ndigits: int, count: int) -> list[float]:
    """Each exact tie with its two nearest neighbours, which must round the *other* way.

    A port that tests `abs(frac - 0.5) < eps` rather than comparing exactly passes the ties and
    fails here, and that is the mistake worth catching: it is invisible on every value a sweep
    draws.
    """
    out: list[float] = []
    for tie in _exact_ties(ndigits, count):
        out += [math.nextafter(tie, -math.inf), tie, math.nextafter(tie, math.inf)]
    return out


def _decimal_traps() -> list[float]:
    """Literals whose decimal look and binary value disagree, which is edge 1's other half.

    Each is a value where `round(x, 2)` is not what the digits suggest, or where the obvious
    `(x * 100).round() / 100` port disagrees with Python. They are the canonical ones rather than
    this repo's, because the trap belongs to CPython's arithmetic and not to this engine's data.
    """
    return [
        0.145,  # exactly 0.14499999999999999000799... -> *down* to 0.14, against how it reads
        0.135,  # exactly 0.13500000000000000888178... -> up to 0.14, so the rule is not "down"
        2.675,  # exactly 2.67499999999999982236431... -> 2.67, where a ties-away scaling gives 2.68
        1.005,  # exactly 1.00499999999999989341858... -> 1.0, the example every bug report cites
        0.215,  # 0.215 * 100.0 is exactly 21.5, so the scaled form rounds *up* and CPython does not
        5.565,  # the same defect in the other direction: CPython 5.57, scaled 5.56
        8.835,
        2.345,
        1234.5678,
        0.0001235,  # 0.0 at two places and 0.0001 at four: the ndigits are not cosmetic
    ]


def _engine_floats(stage_vectors: list[dict[str, Any]], limit: int = 120) -> list[float]:
    """Unrounded values the engine really hands to `round`, read off the stage vectors.

    `spec/vectors/stages/*`'s `measure` stage records each `POSE_MEASUREMENTS` entry's value
    *before* `_measurements` rounds it, which makes it the only place on disk holding this edge's
    real inputs. Reading them costs nothing and needs no captures — the same property the stages
    family itself has (ADR-032 §2).
    """
    seen: set[float] = set()
    for vector in stage_vectors:
        for row in vector.get("stages", {}).get("measure", []):
            value = row.get("value")
            if isinstance(value, float) and math.isfinite(value):
                seen.add(value)
    # Sorted and truncated rather than taken as found: 21 vectors carry a few hundred of these and
    # the marginal one covers nothing the first hundred do not, and a *sorted* slice is stable
    # across a regeneration where a filesystem-ordered one is not.
    return sorted(seen)[:limit]


def _band_edges() -> list[float]:
    """The `ranges.json` band edges, which are what the `:g` sites in `mechanics.py` format.

    Read from the shipped artifact rather than transcribed: `CLAUDE.md`'s rule is that a band
    quoted in a second place is a band that drifts, and a format table is prose enough.
    """
    path = REPO / "src" / "golf_coach" / "analysis" / "benchmarks" / "ranges.json"
    rows = json.loads(path.read_text(encoding="utf-8"))["ranges"]
    edges: list[float] = []
    for row in rows:
        for key in ("low", "high"):
            value = float(row[key])
            for candidate in (value, -value):
                # `mechanics.py:666` formats `abs(band.high)` on a signed band, so the negated
                # edge is as real an input as the edge itself.
                if candidate not in edges:
                    edges.append(candidate)
    return edges


def _sweep(count: int) -> list[float]:
    """A deterministic spread of magnitudes, for the breadth the hand-picked cases lack.

    Seeded and committed rather than generated in the test, because the test is in Rust and has no
    Python to ask. `random.Random` is reproducible across CPython versions by documented
    guarantee, so this regenerates byte-identically anywhere — which is what keeps this family's
    diff empty when nothing has changed.
    """
    rng = random.Random(20260923)  # M22 P3's date, and no other significance
    out: list[float] = []
    for _ in range(count):
        # Exponents from -6 to 8: below the smallest `_norm` this engine measures and above the
        # largest spin value, so the pool brackets every real magnitude rather than sitting on 1.
        mantissa = rng.random() * 2.0 - 1.0
        out.append(mantissa * 10.0 ** rng.randint(-6, 8))
    return out


def _format_rounding(engine_pool: list[float]) -> dict[str, Any]:
    """Edge 1: `round(x)` is half-to-even, and `round(x, n)` is decimal-aware on top of that."""
    cases: list[dict[str, Any]] = []

    # The one-argument form, which returns an `int` and is the dangerous kind of the two:
    # seventeen sites round a frame index, where a one-frame divergence moves which frame a
    # checkpoint is measured on and cascades into every score after it. `alignment.py` holds ten.
    one_arg = _tie_neighbourhood(0, 40)
    one_arg += [-value for value in _tie_neighbourhood(0, 40)]
    one_arg += [0.49999999999999994, -0.49999999999999994]  # C's round() says 1, Python says 0
    one_arg += _sweep(60)
    for value in one_arg:
        cases.append({"value": repr(value), "ndigits": None, "expected": round(value)})

    # The two-argument form. The decimal position is what makes this edge decimal-aware rather
    # than a scaled `f64::round`, so every pool is run at every `ndigits` the engine passes.
    pool = _decimal_traps() + engine_pool + _band_edges() + _sweep(80)
    for ndigits in _ROUND_NDIGITS:
        for value in pool + _tie_neighbourhood(ndigits, 24):
            cases.append(
                {"value": repr(value), "ndigits": ndigits, "expected": repr(round(value, ndigits))}
            )

    # Non-finite input, where `round(x, n)` is the identity and a port that formats and reparses
    # has to not lose that.
    for value in (math.inf, -math.inf, math.nan):
        cases.append({"value": repr(value), "ndigits": 4, "expected": repr(round(value, 4))})

    return _format_vector(
        "format/rounding",
        "`round(x)` and `round(x, n)` over the exact ties, their nearest neighbours, the decimal "
        "traps, the engine's own unrounded values and a seeded magnitude sweep",
        cases,
    )


def _format_fixed(engine_pool: list[float]) -> dict[str, Any]:
    """Edge 2a: `:.Nf` reaches the sentences `docs/CONFORMANCE.md` §3 compares exactly."""
    pool = _decimal_traps() + engine_pool + _band_edges() + _sweep(60)
    cases: list[dict[str, Any]] = []
    for precision in _FIXED_PRECISIONS:
        for value in pool + _tie_neighbourhood(precision, 24):
            cases.append(
                {
                    "value": repr(value),
                    "precision": precision,
                    "expected": f"{value:.{precision}f}",
                }
            )
    # Negative zero, and a value that rounds to one: Python keeps the sign and writes `-0.00`, so
    # a port that normalizes it prints a different sentence than the one a golfer was shown.
    for value in (-0.0, -0.001, -0.4):
        cases.append({"value": repr(value), "precision": 2, "expected": f"{value:.2f}"})
    # Non-finite, and the first of the two places these languages genuinely differ in text:
    # Rust's `{:.2}` of a NaN is `NaN` and Python's is `nan`.
    for value in (math.inf, -math.inf, math.nan):
        for precision in _FIXED_PRECISIONS:
            cases.append(
                {
                    "value": repr(value),
                    "precision": precision,
                    "expected": f"{value:.{precision}f}",
                }
            )
    return _format_vector(
        "format/fixed",
        f"`:.Nf` at the {len(_FIXED_PRECISIONS)} precisions `analysis/` and `feedback/` "
        "interpolate, including the negative zeros and the non-finite text",
        cases,
    )


def _format_general(engine_pool: list[float]) -> dict[str, Any]:
    """Edge 2b: `:g`, which Rust has no equivalent of at all.

    The decade boundaries are the substance. `%g` picks its form from the exponent of the value
    **after** it has been rounded to six significant digits, so `999999.5` prints as `1e+06` and
    not as `999999` or `1000000` — a port that decides the form first gets that boundary wrong in
    one direction, and a port that rounds twice gets it wrong in the other.
    """
    pool = _band_edges() + engine_pool + _sweep(80)
    # The decade walk, both signs, across the two thresholds `%g` actually has: `exp < -4` and
    # `exp >= 6`. `9.999995` is there to carry a value that crosses a decade *by rounding*.
    pool += [sign * 10.0**exp for exp in range(-8, 18) for sign in (1.0, -1.0, 2.5, 9.999995)]
    # Exact ties at the sixth significant digit. In the 1e5 decade those are the half-integers:
    # 123456.5 is representable, ties, and goes *down* to 123456 because 6 is the even digit.
    pool += [float(n) + 0.5 for n in range(123450, 123460)]
    pool += [float(n) * 10.0 for n in range(123450, 123460)]  # the same tie one decade up
    pool += [0.0, -0.0, 1.0, 0.0001, 0.00001, 1e16, 1e100, 1e-100]
    pool += [math.inf, -math.inf, math.nan]
    cases = [{"value": repr(value), "expected": f"{value:g}"} for value in pool]
    return _format_vector(
        "format/general",
        f"`:g` at CPython's default precision of {_G_PRECISION}, walked across both form "
        "thresholds and over the exact ties at the sixth significant digit",
        cases,
    )


def _format_repr(engine_pool: list[float]) -> dict[str, Any]:
    """Edge 4: `str(x)` on a float, which an f-string with no format spec reaches.

    `f"aim under {band.high}"` is three of `mechanics.py`'s sentences, and in Python 3 `str`,
    `repr` and `format(v, "")` are one function for a float — so this one table covers all three
    spellings.

    **Rust's `{}` is the same shortest round-trip digits presented under different rules**, which
    is what makes this edge quiet: every band edge shipping today formats identically in both
    languages, so a port that used `{}` would pass every other vector in this repo. The cases that
    separate them are therefore the substance here, and each is a value a future `ranges.json` row
    could hold: an **integral** value, where CPython's `Py_DTSF_ADD_DOT_0` writes `4.0` and Rust
    writes `4`; and the **exponent window** at `decpt <= -4 || decpt > 16`, which Rust's `{}` does
    not have at all. The decade walk crosses both thresholds from both sides.
    """
    pool = _band_edges() + engine_pool + _sweep(80)
    # Integers as floats, across the width where the fixed form still applies. `1e16` is the first
    # value past the threshold and `1e15` the last before it, so the pair is the boundary itself.
    pool += [float(n) for n in range(0, 11)]
    pool += [10.0**exp for exp in range(0, 18)]
    pool += [-(10.0**exp) for exp in range(0, 18)]
    # The small end, walking `decpt` down through -4 from both sides.
    pool += [sign * 10.0**exp for exp in range(-12, 1) for sign in (1.0, -1.0)]
    pool += [sign * 1.5 * 10.0**exp for exp in range(-12, 18) for sign in (1.0, -1.0)]
    # A signed zero is a real answer here for the same reason it is in `round(x, n)`: CPython keeps
    # the sign and writes `-0.0`, and a port that normalizes it prints a different number.
    pool += [0.0, -0.0, 1e16, 1e-5, 1e-4, 1e100, 1e-100, 5e-324, 1.7976931348623157e308]
    pool += [math.inf, -math.inf, math.nan]
    cases = [{"value": repr(value), "expected": str(value)} for value in pool]
    return _format_vector(
        "format/repr",
        "`str(x)` on a float, walked across both of CPython's `repr` form thresholds "
        "(`decpt <= -4` and `decpt > 16`) and over the integral values Rust renders without a "
        "fractional part",
        cases,
    )


def _format_ordering() -> dict[str, Any]:
    """Edge 3: a stable sort over an insertion-ordered dict decides which *name* a sentence names.

    The cases are the two real shapes plus the third `docs/CONFORMANCE.md` §3 names beside them:
    `joint.py`'s `-abs(share)` over the model's metric order, `trajectory.py`'s `-share` over its
    interval order, and `engine.py:797`'s registry rank, where every unregistered name collides on
    one key and is therefore left in input order.

    **Every case here is a tie, because a case without one gates nothing:** any sort agrees on
    distinct keys, and it is only a tie that reads the insertion order back out.
    """
    model = REPO / "src" / "golf_coach" / "analysis" / "benchmarks" / "joint_model_v1.json"
    metrics = json.loads(model.read_text(encoding="utf-8"))["model"]["metrics"]
    intervals = ["address->top", "top->impact", "impact->finish"]
    registry = ["tempo", "head_sway", "finish_balance", "hip_sway", "hip_shift_at_top"]

    cases: list[dict[str, Any]] = []

    def record(key: str, entries: list[tuple[str, float]]) -> None:
        if key == "neg_abs":
            order = sorted(entries, key=lambda kv: -abs(kv[1]))
        else:
            order = sorted(entries, key=lambda kv: -kv[1])
        cases.append(
            {
                "key": key,
                "entries": [[name, repr(value)] for name, value in entries],
                # Both of what `engine.py` reads: the whole order, and the `next(iter(...))` off
                # the front of it.
                "expected_order": [name for name, _ in order],
                "expected_first": order[0][0],
            }
        )

    # All six shares equal — `joint.py`'s `dict.fromkeys(self.metrics, 0.0)` branch, taken when
    # the distance is zero, where the answer is entirely the model's metric order.
    record("neg_abs", [(metric, 0.0) for metric in metrics])
    # Two tied at the top, which is the case that produces a correct number under a wrong name.
    record("neg_abs", list(zip(metrics, [0.3, 0.3, 0.2, 0.1, 0.05, 0.05], strict=True)))
    # Opposite signs, equal magnitude: `-abs` ties them where `-value` does not, so this is where
    # the two real key functions visibly disagree and a port cannot use one for both.
    signed = [-0.4, 0.4, -0.1, 0.1, 0.0, -0.0]
    record("neg_abs", list(zip(metrics, signed, strict=True)))
    record("neg", list(zip(metrics, signed, strict=True)))
    # The reversed metric order over the same values, which is the assertion that the answer is
    # the insertion order and not something recoverable from the values alone.
    record("neg_abs", list(zip(metrics[::-1], [0.3, 0.3, 0.2, 0.1, 0.05, 0.05], strict=True)))
    # `trajectory.py`'s shape: three intervals, `-share`, two of them tied.
    record("neg", list(zip(intervals, [0.5, 0.25, 0.25], strict=True)))
    record("neg", list(zip(intervals, [0.25, 0.5, 0.25], strict=True)))
    record("neg", [(name, 0.0) for name in intervals])

    # `engine.py:797`: `sorted(unscored, key=lambda e: order.get(e.name, len(order)))`. Three
    # unregistered names collide on `len(registry)` and stay in the order they arrived in.
    mixed = ["hip_sway", "zzz_unregistered", "tempo", "aaa_unregistered", "finish_balance", "mmm"]
    # Every name unregistered, where the sort is the identity and a port that falls back to
    # comparing the names themselves fails outright.
    unknown = ["zzz", "aaa", "mmm"]
    # Unregistered names of **distinct lengths, in neither alphabetical nor length order**, which is
    # the case the first two do not cover: found by mutation, when a rank of `MAX - len(name)`
    # survived both of them. Any fallback that is not one shared constant reorders this list, where
    # above it either ties or happens to agree.
    varied = ["zz", "aaaa", "m", "kkkkkk", "bbb"]
    # And the same names with a registered one in the middle, so a port cannot pass by treating the
    # unregistered block as a separate list to be appended.
    interleaved = ["zz", "hip_sway", "aaaa", "m", "tempo", "kkkkkk", "bbb"]
    for names in (mixed, unknown, varied, interleaved):
        cases.append(
            {
                "key": "registry",
                "registry": registry,
                "names": names,
                "expected_order": sorted(names, key=lambda name: _rank(name, registry)),
            }
        )
    return _format_vector(
        "format/ordering",
        "the tie cases of `joint.py`'s `-abs(share)`, `trajectory.py`'s `-share` and "
        "`engine.py`'s registry rank — every case a tie, because a distinct-keyed sort gates "
        "nothing",
        cases,
    )


def _rank(name: str, registry: list[str]) -> int:
    """`order.get(name, len(order))` — the rank `engine.py` sorts `unscored` by."""
    return registry.index(name) if name in registry else len(registry)


# --------------------------------------------------------------------- format: the parser (M34 P2)
#
# Five more tables, for the CPython the frozen screen parser leans on where the engine does not:
# `repr` on a `str`, float `//`, `str.upper`/`split`/`strip` with `re`'s `\s`, `sum`, and
# `difflib`. They join this family rather than the screen family because every answer in them is
# the *language's* (docs/plans/m34-screen-reader.md, call 2), so they rebuild anywhere and age on
# `python_version`. The parser's own private functions — `normalize_label`, `_first_number`'s
# look-behind, `_sign_from` — are the screen family's `units`, recorded once with it.
#
# **Each names the crate that implements it** (`provenance.implemented_by`), because for the first
# time that is not always `pyfmt`: `difflib_ratio` is `crates/screen`'s, which M34 P3 builds. The
# five engine tables predate the key and are `pyfmt`'s by its absence.
#
# Strings travel as JSON strings, which `json.dumps` writes with `\uXXXX` escapes, so a vector is
# ASCII on disk and a non-ASCII case is still exactly the text CPython was handed. A lone
# surrogate is the one `str` that cannot travel — `serde_json` refuses it and a Rust `String`
# cannot hold one — and no OCR engine produces one, so none is recorded.

#: The parser's edges, as the program plan found them.
_PARSER_EDGES = (
    "docs/plans/m31-m40-shot-first-pivot.md 'What the code says' finding 5; "
    "docs/plans/m34-screen-reader.md planning finding 5 and call 2"
)

#: Every distinct text PaddleOCR read off the thirteen stored bay photos in M34's planning run
#: (2026-10-01, `paddleocr` 3.7.0 with PP-OCRv6 medium), in first-seen order. Transcribed rather
#: than read from `data/`, so this family keeps rebuilding without the capture machine; the screen
#: family's corpus vectors (M34 P4) carry the same boxes with their geometry. It is the realistic
#: half of every table below — `°` on every photo, `ē` on six, `0>1` where the screen says `O>I`.
_OCR_TEXTS: tuple[str, ...] = (
    "Aaron", "Shot Distance", "Carry", "Ball Speed", "Launch Angle", "Club Speed", "Club Path",
    "Club Face Angle", "8.1\xb00>1", "2.8", "131.0", "125.6", "90.7", "20.9", "91.0", "Open",
    "yds", "mph", "Spin Axis", "Impact Position", "Spin", "Smash Factor", "Shot Type",
    "Horizontal Angle", "-2.5\xb0", "5991", "1.00", "CENTER", "SLIGHT", "5.3\xb0L", "FADE", "rpm",
    "131 yds.", "350 yds.", "73 vds.", "HD GOLF", "SHOTDATA", "Custom", "124.2", "121.0", "90.5",
    "23.5", "98.3", "4.6\xb00>1", "8.6\xb0", "Impact Position V", "8100", "0.92", "4.0\xb0R",
    "-9.3\xb0", "71 yds.", "124 yds.", "BALL TRAJECTORY", "C", "SHOT DATA Player 1", "134.1",
    "127.0", "90.9", "20.7", "103.0", "3.2\xb00>1", "6.0", "0.88", "2.8\xb0R", "74 yds.",
    "134 yds.", "NOTIFICATIONS", "BIGSCREEN", "PRINT", "TIPS", "SETTINGS", "BACK", "HELP",
    "ēlo", "MAINMENU", "188.0", "178.2", "114.8", "15.3", "109.5", "2.1\xb01>0", "0.8",
    "1.05", "HEEL", "2.9\xb0R", "109 yds.", "188 yds.", "TA Player 1", "HD", "130.9", "121.8",
    "88.9", "19.5", "87.0", "9.8\xb00>1", "3.1\xb0", "ImpactPosition", "1.02", "6.7\xb0L", "?",
    "69.4", "33.6", "83.8", "3.4", "93.7", "7.7\xb00>1", "7.6\xb0", "0.89", "0.1\xb0L", "18.yds.",
    "69 yds.", "OT DATA Player 1", "H", "Custo", "135.1", "124.7", "91.1", "18.2", "98.2",
    "4.5\xb00>1", "0.93", "1.0\xb0R", "I", "72 yds.", "135 yds.", "T DATA Player 1", "131.5",
    "126.1", "89.8", "22.4\xb0", "103.6", "1.1\xb00>1", "3.9", "0.87", "TOE", "73 yds.",
    "132 yds.", "BIG SCREEN", "109.7", "101.9", "78.9", "22.4", "103.8", "3.7\xb00>1", "10.2",
    "0.76", "HARD", "6.5\xb0R", "56 yds.", "110 yds.", "HDG", "159.2", "137.7", "11.4", "105.5",
    "0.5\xb01>0", "2.4\xb0", "Closed", "0.98", "1.8\xb0L", "DRAW", "80 yds.", "159 yds.", "154.1",
    "144.5", "100.9", "17.0", "105.7", "2.3\xb00>1", "6.9", "0.95", "4.6\xb0R", "85 yds.",
    "154 yds.", "SHOTDATA Player 1", "122.7", "107.2", "85.1", "15.9", "85.0", "1.3\xb00>1",
    "1.3\xb0", "0.0\xb0L", "61 yds.", "123 yds.", "1", "146.0", "95.9", "17.5", "90.2", "135.8",
    "o", "1.3\xb01>0", "4.6", "1.06", "5.9\xb0R", "79 yds.", "146 yds.",
)

#: The tile M34 adds to the Rust fork of the profile (decision 1), which the frozen file never
#: gains. Its labels are inputs here because the fork matches every box against it from P8 on.
_FORK_ONLY_LABELS = ("Impact Position V",)

#: Strings no photo has produced yet, each standing for one way a `str` operation in Rust can part
#: company with CPython's. Shared by the `repr` and text tables, because a string that separates the
#: two languages under one operation is worth running through the others.
_CRAFTED_TEXTS: tuple[str, ...] = (
    "",
    " ",
    # Quote choice: `'` alone flips CPython to double quotes; both keep single and escape `'`.
    "'",
    '"',
    "it's",
    'say "hi"',
    "both ' and \"",
    "back\\slash",
    # The three named escapes, then the `\xhh` C0 controls and DEL around them.
    "tab\there",
    "new\nline",
    "cr\rhere",
    "\x00",
    "\x07\x08",
    "\x1b[0m",
    "\x7f",
    # Python's whitespace has four code points Rust's does not: the C0 separators.
    "a\x1cb\x1dc\x1ed\x1fe",
    " \x1c\x1d\x1e\x1f ",
    "\t\n\x0b\x0c\r",
    # Non-ASCII whitespace both languages agree on, and two look-alikes that are not whitespace.
    "a\x85b",
    "a\xa0b",
    "a b",
    "a b",
    "a b c",
    "a b c",
    "a　b",
    "a​b",
    "a﻿b",
    "a᠎b",
    "  lead and trail  ",
    "multiple   spaces",
    # `repr`'s printable test on non-ASCII: C1 controls, a soft hyphen (Cf), the zero-width
    # joiners, a bidi override, private use, an unassigned code point and a noncharacter all
    # escape; a combining mark — even at the very start — and astral emoji do not.
    "\x80\x9f",
    "\xad",
    "‌‍",
    "‮",
    "",
    "͸",
    "￿",
    "\U000e0001",
    "\U0010ffff",
    "é",
    "́",
    "\U0001f600",
    # Characters a screen or an OCR engine plausibly puts beside a number.
    "\xb0",
    "ē",
    "—",
    "–",
    "→",
    "Ω",
    "中文",
    # `upper`'s full (SpecialCasing) mappings, where one character becomes two or three.
    "\xdf",
    "stra\xdfe",
    "ﬁnal",
    "ﬀ",
    "ŉ",
    "ǰ",
    "ΐ",
    "և",
    "ᾳ",
    "ᾼ",
    # Single-character mappings that land somewhere surprising, and two that stay put.
    "ı",
    "i̇",
    "\xb5",
    "\xff",
    "ſ",
    "ǅ",
    "ა",
    "ͅ",
    "K",
    "ẞ",
)


def _unique(texts: Any) -> list[str]:
    """First-seen order with repeats dropped, so a regeneration is byte-stable."""
    out: list[str] = []
    for text in texts:
        if text not in out:
            out.append(text)
    return out


def _profile_names() -> list[str]:
    """Every label and alias the frozen `hd_golf` profile matches a box against, plus the fork's.

    Read from the shipped profile rather than transcribed, as `_band_edges` reads `ranges.json`.
    """
    profile = load_profile("hd_golf")
    labels = [name for field in profile.fields for name in (field.label, *field.aliases)]
    return _unique([*labels, *_FORK_ONLY_LABELS])


def _profile_texts() -> list[str]:
    """What the profile puts in front of a `str` operation: the names (`{missing!r}` in a
    warning), the title (`{profile.title!r}`) and the blank markers (`_is_blank`'s `\\s` strip).
    """
    profile = load_profile("hd_golf")
    return _unique([*_profile_names(), profile.title, *profile.blank_markers])


def _format_str_repr() -> dict[str, Any]:
    """`repr(s)` on a `str`, which every `{text!r}` in a parser warning reaches.

    Warnings are compared exactly (`docs/CONFORMANCE.md` §3), and three of the parser's carry a
    `!r` — a cell's text, a missing label, the title — so a port's quote choice and escapes are
    in a golfer-visible string. Rust's `{:?}` is the nearest call and disagrees on the quote, on
    the escape spelling and on which characters are printable, which `pyfmt::str_repr` measures.
    """
    cases = [
        {"value": text, "expected": repr(text)}
        for text in _unique([*_CRAFTED_TEXTS, *_OCR_TEXTS, *_profile_texts()])
    ]
    return _format_vector(
        "format/str_repr",
        "`repr(s)` on a `str`: quote choice, backslash escapes, `\\xhh`/`\\uhhhh`/`\\Uhhhhhhhh` "
        "for the non-printable, printable non-ASCII kept — over every box text of the planning "
        "run, the frozen profile's strings and crafted cases",
        cases,
        edges=_PARSER_EDGES,
        implemented_by="pyfmt",
    )


def _format_floor_div() -> dict[str, Any]:
    """`a // b` on floats, which `_Cell.text` keys its line order on: `int(center_y // bucket)`.

    CPython's float `//` is `fmod`-based, not `floor(a / b)`, and the two part company exactly
    where the parser lives: a center that sits on a multiple of a bucket the binary point cannot
    represent. `21.0 // 4.2` is `4.0` in CPython, while `21.0 / 4.2` rounds to exactly `5.0` —
    a label 7 px tall, and a value box whose center is at y=21. Over the integer-pixel grid
    PaddleOCR returns (centers on the half-pixel up to 2,000, label heights 1-80 px) that is 300
    of 320,080 pairs, every one of them recorded here; over the same span drawn uniformly at
    random it is none of 200,000, which is why a sweep alone would gate nothing.
    """
    pairs: list[tuple[float, float]] = [
        # The classic: the quotient rounds *up* to an integer the true value is below.
        (1.0, 0.1),
        (-1.0, 0.1),
        (0.3, 0.1),
        (0.7, 0.1),
        # Signed zeros, which `//` keeps through `copysign` where a floor of a division agrees.
        (0.0, 0.6),
        (-0.0, 0.6),
        (0.0, -0.6),
        (-0.0, -0.6),
        # Negatives in each position, where the remainder's sign decides the correction.
        (-5.5, 2.0),
        (5.5, -2.0),
        (-5.5, -2.0),
        (-21.0, 4.2),
        (21.0, -4.2),
        # The `max(..., 1e-6)` floor on a zero-height label, which makes the bucket tiny.
        (0.0, 1e-6),
        (1e-7, 1e-6),
        (1.5e-6, 1e-6),
        (123.25, 1e-6),
        (500.5, 1e-6),
        (1999.5, 1e-6),
        # Exact multiples of a representable bucket, which both forms agree on.
        (36.0, 18.0),
        (35.999999999999996, 18.0),
    ]
    # Every pair on the integer-pixel grid where the obvious port disagrees.
    for height in range(1, 81):
        bucket = max(height * _LINE_BUCKET, 1e-6)
        for half_pixels in range(0, 4001):
            center = half_pixels / 2
            if math.floor(center / bucket) != center // bucket:
                pairs.append((center, bucket))
    # And the ordinary case, integer and fractional geometry alike: VisionKit's boxes arrive in
    # normalized coordinates, so a phone's centers and heights are not on any pixel grid.
    rng = random.Random(20261001)  # M34 P2's date, and no other significance
    for _ in range(120):
        center = rng.randrange(0, 4001) / 2
        pairs.append((center, max(rng.randrange(1, 81) * _LINE_BUCKET, 1e-6)))
    for _ in range(120):
        pairs.append((rng.uniform(0.0, 2000.0), max(rng.uniform(0.0, 80.0) * _LINE_BUCKET, 1e-6)))

    cases = []
    for a, b in pairs:
        quotient = a // b
        cases.append(
            {
                "a": repr(a),
                "b": repr(b),
                "expected": repr(quotient),
                # What the parser actually keys on. Python's `int` has no signed zero, so this is
                # the comparison that is allowed to be blind to one — the float form above is not.
                "as_int": int(quotient),
            }
        )
    return _format_vector(
        "format/floor_div",
        "float `a // b` and `int(a // b)`, CPython's `fmod`-based floor division: every pair on "
        "the integer-pixel grid where `floor(a / b)` disagrees, signed zeros, the 1e-6 bucket "
        "floor, and a seeded sweep of ordinary geometry",
        cases,
        edges=_PARSER_EDGES,
        implemented_by="pyfmt",
    )


def _format_text_case() -> dict[str, Any]:
    """`str.upper`, `str.split`, `str.strip` and `re.sub(r"\\s+", "", s)` over Unicode.

    Four call sites in the parser: `normalize_label` and the text tiles upper-case, the text tiles
    then `split()` and rejoin, `_Cell.text` strips each box, and `_is_blank` deletes `\\s+`. All
    four lean on one whitespace set, so the table also records that set **whole** — every code
    point `str.isspace` accepts, and every one `re`'s `\\s` matches — for the Rust test to check
    against all 1,114,112 code points rather than against the ones somebody thought to try.
    """
    pool = _unique([*_CRAFTED_TEXTS, *_OCR_TEXTS, *_profile_texts()])
    cases: list[dict[str, Any]] = []
    for text in pool:
        cases.append({"op": "upper", "value": text, "expected": text.upper()})
    for text in pool:
        cases.append({"op": "split", "value": text, "expected": text.split()})
    for text in pool:
        cases.append({"op": "strip", "value": text, "expected": text.strip()})
    for text in pool:
        cases.append({"op": "strip_space", "value": text, "expected": re.sub(r"\s+", "", text)})
    everything = range(sys.maxunicode + 1)
    cases.append(
        {
            "op": "space_set",
            "source": "str.isspace",
            "expected": [cp for cp in everything if chr(cp).isspace()],
        }
    )
    space = re.compile(r"\s")
    cases.append(
        {
            "op": "space_set",
            "source": "re \\s",
            "expected": [cp for cp in everything if space.fullmatch(chr(cp))],
        }
    )
    return _format_vector(
        "format/text_case",
        "`str.upper()`, `str.split()`, `str.strip()` and `re.sub(r'\\s+', '', s)` over the "
        "planning run's box texts, the profile's strings and crafted Unicode, plus the whole "
        "whitespace set as `str.isspace` and `re`'s `\\s` each define it",
        cases,
        edges=_PARSER_EDGES,
        implemented_by="pyfmt",
    )


def _float32(value: float) -> float:
    """`value` as the nearest float32, widened back — what PaddleOCR's confidences are."""
    widened: float = struct.unpack("f", struct.pack("f", value))[0]
    return widened


def _format_sum(stage_vectors: list[dict[str, Any]]) -> dict[str, Any]:
    """`sum()` over floats, which has been **compensated** since CPython 3.12 (Neumaier).

    `sum([0.1] * 10)` is `1.0` on the recording interpreter, where a left-to-right fold gives
    `0.9999999999999999`. The parser's `_score` takes `sum(ocr) / len(ocr)`, which reaches
    `round(confidence, 3)` and the `< min_confidence` bool, so this is not a last-bit nicety.

    **Where it bites is not where it looks like it would.** A real OCR confidence is a float32
    widened to float64 (PaddleOCR's, and VisionKit's `VNConfidence` too), so it carries 24
    significant bits and fifty of them sum *exactly* in a float64: compensation cannot move the
    answer, and on none of the planning run's fifteen photos nor 10,000 seeded photo-sized lists
    does it. The synthetic screens are the other way round — `build_screen` gives every box the
    float64 `0.95`, and `sum([0.95] * k)` leaves a left fold for 50 of the box counts 1-60 — so
    the screen family's synthetic vectors are what this edge will actually gate. Both shapes are
    here, and so are the engine's six checkpoint scores per vector (`scoring.mean_percent` is a
    `sum()` too), read off the stages family the way `_engine_floats` reads it.
    """
    lists: list[list[float]] = [
        [0.1] * 10,
        [0.1, 0.2, 0.3],
        [1e16, 1.0, -1e16],
        [1.0, 1e100, 1.0, -1e100],
        [0.5],
        # Signed zero: CPython starts from the `int` 0, and `0 + -0.0` is `0.0`.
        [-0.0],
        [-0.0, -0.0],
        [-1.5, 1.5],
        # Non-finite, where the compensation term is dropped rather than turning `inf` into a NaN.
        [math.inf, 1.0],
        [1.0, math.inf],
        [math.inf, -math.inf],
        [math.nan, 1.0],
        [1e308, 1e308],
        [1e308, 1e308, -1e308],
        [1.7976931348623157e308, 1e292, -1.7976931348623157e308],
        # Magnitudes that swap order mid-list, which is the branch Neumaier adds to Kahan.
        [1.0, 1e-16, 1e-16, 1e-16, 1e-16],
        [1e-16, 1e-16, 1e-16, 1e-16, 1.0],
        [3.0, -1e-16, 1e20, -1e20, 1e-16],
    ]
    for vector in stage_vectors:
        scores = [
            row["score"]["score"]
            for row in vector.get("stages", {}).get("checkpoints", [])
            if row.get("score") is not None
        ]
        if scores:
            lists.append(scores)
    # The synthetic screens' constant confidence, at every box count a screen could carry.
    lists += [[0.95] * count for count in range(1, 61)]
    rng = random.Random(20261001)
    # Float64 confidences, where compensation moves the answer on about three lists in five...
    for _ in range(40):
        lists.append([0.85 + 0.15 * rng.random() for _ in range(rng.randint(35, 50))])
    # ...and float32 ones, where it never does: the agreement is the case, not a gap in it.
    for _ in range(10):
        lists.append([_float32(0.85 + 0.15 * rng.random()) for _ in range(rng.randint(35, 50))])

    cases = [
        {"values": [repr(value) for value in values], "expected": repr(sum(values))}
        for values in lists
    ]
    return _format_vector(
        "format/sum",
        "`sum()` over float lists at the recording interpreter, compensated since CPython 3.12: "
        "the textbook cancellations, signed zeros, non-finite input, the engine's checkpoint "
        "scores, the synthetic screens' constant 0.95, and seeded float64 and float32 "
        "confidence lists the size of a photo's boxes",
        cases,
        edges=_PARSER_EDGES,
        implemented_by="pyfmt",
    )


#: `difflib` cases no photo produced: an empty side, `find_longest_match`'s tie-breaking (the
#: earliest `i`, then the earliest `j`), repeated characters, and the label pairs the tie rule is
#: argued from (M34 planning finding 3). Then `autojunk`, which only fires once `len(b) >= 200`:
#: `Q` is popular in the third pair, so the match it would make is dropped — but in the first,
#: `find_longest_match`'s extension step re-grows a match across popular characters, because it
#: only refuses `isjunk` junk. Every label is short, so whether P3 ports this or refuses such
#: input is its call; the cases are here so either answer is checked.
_DIFFLIB_CRAFTED: tuple[tuple[str, str], ...] = (
    ("", ""),
    ("", "CARRY"),
    ("CARRY", ""),
    ("CARNY", "CARRY"),
    ("BOUNCE ROLL", "BOUNCE AND ROLL"),
    ("IMPACT POSITION", "IMPACT POSITION V"),
    ("IMPACT POSITION V", "IMPACT POSITION"),
    ("IMPACTPOSITION", "IMPACT POSITION"),
    ("IMPACTPOSITION", "IMPACT POSITION V"),
    ("IMPACT POSITION Y", "IMPACT POSITION"),
    ("IMPACT POSITION Y", "IMPACT POSITION V"),
    ("LMPACT POSITION", "IMPACT POSITION"),
    ("LMPACT POSITION", "IMPACT POSITION V"),
    ("CLUB SPEED", "CLUB PATH"),
    ("SPIN", "SPIN AXIS"),
    ("SPIN AXIS", "SPIN"),
    ("ABCXABC", "ABC"),
    ("ABC", "ABCXABC"),
    ("XABCYABCZ", "ABCABC"),
    ("ABCD", "DCBA"),
    ("ABXCD", "ABYCD"),
    ("AAAA", "AA"),
    ("AA", "AAAA"),
    ("ABABAB", "BABA"),
    ("AAABBB", "BBBAAA"),
    # Characters, not bytes: `normalize_label` only ever hands over ASCII, but a matcher that
    # counted UTF-8 bytes would pass every label and fail here.
    ("\xb0C", "C"),
    ("ĒLO", "ELO"),
    ("\xc9\xc9", "\xc9"),
    ("AB", "AB" * 100),
    ("ABAB", "AB" * 99),
    ("QQQ", "XYZ" + "Q" * 197),
    ("XYZ", "XYZ" + "Q" * 197),
)


def _format_difflib_ratio() -> dict[str, Any]:
    """`SequenceMatcher(None, a, b).ratio()`, which `ProfileField.matches` decides a label by.

    `a` is a box's text and `b` a label or alias, both through `normalize_label`, which is the pair
    `matches` hands to `difflib` — so every box of the planning run against every name the frozen
    profile carries, plus the fork's `Impact Position V`. A box that normalizes to nothing never
    reaches `difflib` (`matches` returns 0.0 first), so it is not a case here.

    **The implementation is `crates/screen`'s, not `pyfmt`'s** (§M34 puts `difflib.rs` there), and
    M34 P3 builds it; `ratio` is exact arithmetic on integer counts, so it is compared on bits.
    """
    names = _unique(normalize_label(name) for name in _profile_names())
    boxes = _unique(normalize_label(text) for text in _OCR_TEXTS)
    pairs = [*_DIFFLIB_CRAFTED, *((a, b) for a in boxes if a for b in names)]
    cases = [
        {"a": a, "b": b, "expected": repr(SequenceMatcher(None, a, b).ratio())} for a, b in pairs
    ]
    return _format_vector(
        "format/difflib_ratio",
        "`difflib.SequenceMatcher(None, a, b).ratio()` over every normalized box text of the "
        "planning run against every normalized label and alias of the frozen profile and the "
        "fork's `Impact Position V`, plus tie-breaking, repeated-character and `autojunk` cases",
        cases,
        edges=_PARSER_EDGES,
        implemented_by="screen",
    )


# ------------------------------------------------------------- format: the many-shot layer (M36 P4)
#
# Three more tables, for the CPython and pydantic the career aggregates, the stores and the five
# report scripts lean on where neither the engine nor the parser did. Two are `pyfmt`'s: `:.Ng` at a
# precision other than `%g`'s default, and `str.lower()`. The third is **`crates/contracts`'**, the
# first format table whose answer is pydantic's rather than the interpreter's: how a `datetime`
# field reads and writes, which every manifest, golfer, bag and shot on disk spells. They join this
# family for the parser's reason (docs/plans/m34-screen-reader.md, call 2): every answer is the
# language's or the library's, so they rebuild anywhere and age on `python_version` (and, for the
# timestamp table, the `pydantic_version` it also carries).
#
# What M36 found and these do not cover is named where it was found rather than tabled here: the
# float `**` choice is P11's to measure, and the report padding is std formatting by code point
# (the M36 plan's P3 finding 5).

#: Where these three tables' edges are written down.
_CAREER_EDGES = (
    "docs/plans/m36-many-shot-layer.md 'What the planning read found' finding 5, "
    "and its P1 and P3 findings 5"
)

#: The precisions `:.Ng` is interpolated with across M36's surface, measured with `grep -rnoE
#: '\.[0-9]+g\}' src/golf_coach scripts` and not chosen: `analysis/dispersion.py:208-209`, the
#: drift caveat's two spreads. (`scripts/pose_replay.py`'s `.3g` and `.6g` are a lab tool's and
#: port with it, not here.) `general.json` is `%g`'s default precision of 6 and stays the gate for
#: that; this table is every other precision, so the default is not tabled twice.
_G_PRECISIONS = (3,)


def _format_general_precision(engine_pool: list[float]) -> dict[str, Any]:
    """`:.Ng` at the precisions the many-shot layer formats with — `%g` with `P` other than 6.

    The same two thresholds as `general.json`, moved: exponent form when the exponent of the value
    *after* rounding to `P` significant digits is below -4 or at least `P`. At `P = 3` that puts
    the boundary at 1000 rather than at 1e6, so `999.5` prints `1e+03` — a spread in yards or rpm
    is a magnitude this caveat really prints — and the exact ties sit at the third significant
    digit: `12.25` goes down to `12.2` and `12.75` up to `12.8`, half to even.
    """
    pool = _band_edges() + engine_pool + _sweep(80)
    # The decade walk across both thresholds, and values that cross a decade *by rounding* at the
    # third digit (`9.996` is `10`, `999.6` is `1e+03`), each with both signs.
    pool += [
        sign * 10.0**exp
        for exp in range(-8, 8)
        for sign in (1.0, -1.0, 2.5, -2.5, 9.996, -9.996, 9.994)
    ]
    # Exact ties at the third significant digit, one decade at a time: odd eighths in [1, 10),
    # quarter-past and quarter-to in [10, 20), half-integers in the hundreds and fives in the
    # thousands. Every one is representable, so the tie is real and not a decimal look.
    pool += [k / 8 for k in range(9, 80, 2)]
    pool += [n + f for n in range(10, 20) for f in (0.25, 0.75)]
    pool += [n + 0.5 for n in range(120, 130)]
    pool += [float(n * 10 + 5) for n in range(120, 130)]
    # And the tie that is also the threshold: `999.5` ties up to `1e+03`, `998.5` down to `998`.
    pool += [999.5, -999.5, 998.5]
    pool += [0.0, -0.0, 0.0001, 0.00001, 1e16, 1e-100, math.inf, -math.inf, math.nan]
    cases = [
        {"value": repr(value), "precision": precision, "expected": f"{value:.{precision}g}"}
        for precision in _G_PRECISIONS
        for value in pool
    ]
    return _format_vector(
        "format/general_precision",
        f"`:.Ng` at {', '.join(str(p) for p in _G_PRECISIONS)}, the precisions M36's surface "
        "formats with: walked across both form thresholds, through the values that cross a decade "
        "by rounding, and over the exact ties at the last significant digit",
        cases,
        edges=_CAREER_EDGES,
        implemented_by="pyfmt",
    )


#: `str.lower()`'s inputs on the many-shot surface: golfer names (`golfer.py::slugify`, the golfer
#: store's `list_all` sort) and club names (`club.py::_normalize`, `club_spec.py::_normalize`), and
#: Greek capital sigma in each context CPython's `Final_Sigma` rule tells apart — the one place
#: `lower` reads a character's neighbours, which `upper` never does.
_LOWER_TEXTS: tuple[str, ...] = (
    # Final sigma: a `Σ` preceded by a cased letter and not followed by one lowers to `ς`, with
    # case-ignorable characters (an apostrophe, a combining mark, a soft hyphen) skipped both ways.
    "Σ",
    "ΣΣ",
    "ΑΣ",
    "ΑΣΣ",
    "ΑΣΑ",
    "ΑΣ.",
    "Α.Σ",
    "Α'Σ",
    "ΑΣ'",
    "ΑΣ'Α",
    "1Σ",
    "ΑΣ1",
    "ΑΣ́",
    "ΆΣ",
    "ΑΣ\xad",
    "ΑΣ\xadΑ",
    "AΣ",
    "ΟΔΟΣ ΟΔΟΣ",
    "ΣΩΚΡΆΤΗΣ",
    "Σωκράτης",
    # Names, the slug's input: accents, a dotted capital I that lowers to two code points, the
    # three letterlike symbols that lower *into* ASCII or Greek, titlecase digraphs, a roman
    # numeral, a circled letter and full-width forms.
    "Aaron Sierra",
    "AARON",
    "María",
    "MARÍA",
    "José-María Ruiz",
    "ÉAMON",
    "NGUYỄN",
    "İnci",
    "İ",
    "Kelvin",
    "Å",
    "Ωmega",
    "STRASSE",
    "Straße",
    "ẞ",
    "SØREN",
    "ﬁne",
    "ǄEMAL",
    "ǅemal",
    "ǈ",
    "ǋ",
    "ǲ",
    "Ⅻ",
    "Ⓐ",
    "Ａaron",
    "ＡＢＣ",
    "ZOË",
    "ДМИТРИЙ",
    "Ἀθῆναι",
    "ᾼ",
    # Club names, as a golfer types one at `--club`.
    "7I",
    "Driver",
    "3W",
    "PW",
    "Pitching Wedge",
    "HYBRID 4",
    "60\xb0",
)


def _format_lower() -> dict[str, Any]:
    """`str.lower()`, over strings and over the whole code space.

    Rust's `str::to_lowercase` is the same full mapping (`İ` to `i` plus a combining dot, both
    applying `SpecialCasing.txt`'s unconditional rules) and the same `Final_Sigma` context, so
    `pyfmt::lower` is a function of its own for `upper`'s reason: the two drift where their Unicode
    versions do, and the day they do the fix has an address. The `lower_map` case records every
    code point CPython lowers to something else, with what it lowers to, so the Rust test checks
    every one of them rather than the ones somebody thought to type.
    """
    pool = _unique([*_CRAFTED_TEXTS, *_OCR_TEXTS, *_profile_texts(), *_LOWER_TEXTS])
    cases: list[dict[str, Any]] = [
        {"op": "lower", "value": text, "expected": text.lower()} for text in pool
    ]
    cases.append(
        {
            "op": "lower_map",
            "expected": [
                [cp, chr(cp).lower()]
                for cp in range(sys.maxunicode + 1)
                if chr(cp).lower() != chr(cp)
            ],
        }
    )
    return _format_vector(
        "format/lower",
        "`str.lower()` over golfer and club names, the parser tables' strings and every "
        "`Final_Sigma` context, plus every code point CPython lowers to something else",
        cases,
        edges=_CAREER_EDGES,
        implemented_by="pyfmt",
    )


#: Offsets the timestamp sweep writes every local time in, in minutes east of UTC: UTC, the two the
#: career family carries (`+05:30`, `-05:00`), a half-hour and a quarter-hour zone, both ends of the
#: real world (`+14:00`, `-12:00`), and the widest pydantic accepts (`±23:59`).
_TIMESTAMP_OFFSETS = (0, 330, -300, -480, 120, -210, 345, 840, -720, 1439, -1439)

#: Local times the sweep writes in each offset, as `datetime` fields: a whole second, a fraction
#: with trailing zeros (`.120000`, which pydantic keeps), six digits, the smallest and largest
#: fraction, the first and last instants of a year, leap days in each kind of century, and the
#: epoch from both sides (negative microseconds are where a truncating division goes wrong).
_TIMESTAMP_LOCALS: tuple[tuple[int, ...], ...] = (
    (2026, 8, 4, 12, 0, 0, 0),
    (2026, 8, 4, 12, 0, 0, 120000),
    (2026, 8, 4, 12, 0, 0, 500000),
    (2026, 8, 4, 12, 0, 0, 100000),
    (2026, 8, 4, 12, 0, 0, 1),
    (2026, 8, 4, 12, 0, 0, 999999),
    (2026, 8, 10, 1, 38, 46, 828488),
    (2026, 8, 23, 4, 49, 25, 21296),
    (2026, 1, 1, 0, 0, 0, 0),
    (2025, 12, 31, 23, 59, 59, 999999),
    (2024, 2, 29, 12, 0, 0, 0),
    (2000, 2, 29, 0, 0, 0, 0),
    (1900, 2, 28, 23, 59, 59, 0),
    (1970, 1, 1, 0, 0, 0, 0),
    (1969, 12, 31, 23, 59, 59, 999999),
    (1969, 12, 31, 23, 59, 59, 500000),
)

#: Spellings pydantic reads to an instant it then writes differently: `+00:00` and `-00:00` for
#: `Z`, a fraction shorter than six digits, and a zero fraction it drops. Each is inside the
#: grammar `contracts::time` accepts, so each is a case both sides must re-spell identically.
_TIMESTAMP_RESPELLED = (
    "2026-08-04T12:00:00+00:00",
    "2026-08-04T12:00:00-00:00",
    "2026-08-04T12:00:00.1Z",
    "2026-08-04T12:00:00.12Z",
    "2026-08-04T12:00:00.120Z",
    "2026-08-04T12:00:00.1200Z",
    "2026-08-04T12:00:00.12000Z",
    "2026-08-04T12:00:00.000000Z",
    "2026-08-04T12:00:00.5+00:00",
    "2026-08-04T12:00:00.5-00:00",
    "2026-08-04T17:30:00.25+05:30",
    "2026-08-04T12:00:00.000+05:30",
    "2026-08-04T12:00:00.0-08:00",
    "2026-08-04T03:00:00.99999-09:00",
    "1969-12-31T23:59:59.9Z",
    "0001-01-01T00:00:00.000000+00:00",
)

#: Strings pydantic refuses, so `contracts::time` must too. What pydantic *accepts* and the Rust
#: grammar refuses — naive, a space or a lowercase `t`, `+0530`, no seconds, seven fraction digits,
#: a comma, a Unix number — is the M36 plan's call 9 divergence and is pinned in
#: `crates/contracts/tests/time.rs`, not here: this table records only what the two agree on.
_TIMESTAMP_REFUSED = (
    "",
    "not a timestamp",
    "2026-08-04T24:00:00Z",
    "2026-08-04T23:59:60Z",
    "2026-08-04T12:60:00Z",
    "2026-02-29T12:00:00Z",
    "1900-02-29T12:00:00Z",
    "2026-04-31T00:00:00Z",
    "2026-13-01T00:00:00Z",
    "2026-00-01T00:00:00Z",
    "2026-08-00T00:00:00Z",
    "0000-01-01T00:00:00Z",
    "2026-08-04T12:00:00+24:00",
    "2026-08-04T12:00:00+23:60",
    "2026-08-04T12:00:00.Z",
    "2026-8-04T12:00:00Z",
    "+2026-08-04T12:00:00Z",
    "2026-08-+4T12:00:00Z",
    "2026-08-04T12:00:00+05:30:15",
    "2026-08-04T12:00:00 Z",
    "2026-08-04T12:00:00+05",
    "2026-08-04T12:00:00ZZ",
    " 2026-08-04T12:00:00Z",
    "１９７０-01-01T00:00:00Z",
)


def _format_timestamp() -> dict[str, Any]:
    """A pydantic `datetime` field: what it reads, how it writes, and three things read off it.

    Every timestamp on disk is a pydantic `datetime` written by `model_dump_json`: `Z` for UTC and
    `±HH:MM` otherwise, the fraction as six digits whenever it is non-zero (trailing zeros kept)
    and dropped when it is zero. `contracts::time::Timestamp` writes that spelling back, keeping
    the offset it read. Each row also records what the many-shot layer reads off a parsed value:

    - `isoformat` — `datetime.isoformat()`, which `storage/corpus.py::_arrival` sorts on **as a
      string** (the M36 plan's P1 finding 5). It is pydantic's spelling with `+00:00` for `Z`.
    - `date` — `f"{dt:%Y-%m-%d}"` in the value's own offset (`club_profile.py`'s bag entry and
      bag-changed caveat). `null` below year 1000, where `%Y` is the C library's `strftime` and so
      the platform's to pad; nothing on disk is that old.
    - `epoch_us` — the instant, as whole microseconds since 1970-01-01T00:00:00Z. Python compares
      two aware datetimes by instant alone, so this one integer is the whole ordering and equality
      a port owes: equal instants in different offsets are equal, and hash alike.

    Three kinds of row: `written` (a value the sweep built, in pydantic's spelling, which reads
    back to itself), `respelled` (another spelling pydantic reads, with what it writes instead),
    and `refused` (every answer `null`). The builder asserts each row is the kind it claims.
    """
    from datetime import UTC, datetime, timedelta, timezone

    import pydantic
    from pydantic import BaseModel, ValidationError

    class _At(BaseModel):
        at: datetime

    epoch = datetime(1970, 1, 1, tzinfo=UTC)

    def read(text: str) -> datetime | None:
        try:
            return _At.model_validate({"at": text}).at
        except ValidationError:
            return None

    def written_by_pydantic(value: datetime) -> str:
        spelled: str = json.loads(_At(at=value).model_dump_json())["at"]
        return spelled

    def row(kind: str, text: str) -> dict[str, Any]:
        parsed = read(text)
        if parsed is None:
            assert kind == "refused", f"pydantic refuses {text!r}, which is filed as {kind}"
            return {
                "kind": kind,
                "value": text,
                "pydantic": None,
                "isoformat": None,
                "date": None,
                "epoch_us": None,
            }
        assert kind != "refused", f"pydantic reads {text!r}, so it is not a refusal both make"
        assert parsed.tzinfo is not None, f"{text!r} reads naive, which is a divergence, not a row"
        spelled = written_by_pydantic(parsed)
        assert (spelled == text) == (kind == "written"), f"{text!r} filed as {kind}: {spelled!r}"
        again = read(spelled)
        assert again == parsed and again is not None and again.utcoffset() == parsed.utcoffset()
        return {
            "kind": kind,
            "value": text,
            "pydantic": spelled,
            "isoformat": parsed.isoformat(),
            "date": f"{parsed:%Y-%m-%d}" if parsed.year >= 1000 else None,
            "epoch_us": (parsed - epoch) // timedelta(microseconds=1),
        }

    def zone(minutes: int) -> timezone:
        return UTC if minutes == 0 else timezone(timedelta(minutes=minutes))

    built = [
        datetime(*fields, tzinfo=zone(minutes))
        for fields in _TIMESTAMP_LOCALS
        for minutes in _TIMESTAMP_OFFSETS
    ]
    built += [
        # The career family's own offsets on its own dates, and the instants where a date in the
        # value's offset and the date in UTC disagree, from both sides of midnight.
        datetime(2026, 8, 20, 23, 30, tzinfo=zone(-300)),
        datetime(2026, 8, 10, 17, 39, tzinfo=zone(330)),
        datetime(2026, 8, 3, 14, 1, tzinfo=zone(120)),
        datetime(2026, 8, 11, 8, 0, tzinfo=zone(-300)),
        datetime(2025, 12, 31, 19, 0, tzinfo=zone(-300)),
        datetime(2026, 1, 1, 5, 29, 59, tzinfo=zone(330)),
        # The range's ends, in UTC only: an offset there would put the instant outside it.
        datetime(1, 1, 1, tzinfo=UTC),
        datetime(999, 12, 31, 23, 59, 59, 999999, tzinfo=UTC),
        datetime(1000, 1, 1, tzinfo=UTC),
        datetime(9999, 12, 31, 23, 59, 59, 999999, tzinfo=UTC),
    ]
    cases = [row("written", written_by_pydantic(value)) for value in built]
    cases += [row("respelled", text) for text in _TIMESTAMP_RESPELLED]
    cases += [row("refused", text) for text in _TIMESTAMP_REFUSED]
    vector = _format_vector(
        "format/timestamp",
        f"a pydantic `datetime` field: a sweep of instants in {len(_TIMESTAMP_OFFSETS)} offsets "
        "as `model_dump_json` writes them, the other spellings it reads and re-spells, and "
        "strings it refuses — each "
        "with `isoformat()`, `%Y-%m-%d` in its own offset, and the instant in microseconds",
        cases,
        edges=_CAREER_EDGES,
        implemented_by="contracts",
    )
    vector["provenance"]["pydantic_version"] = pydantic.VERSION
    return vector


# --------------------------------------------------------------------------- screen (M34 P4)
#
# The frozen screen parser, recorded once: step 1 of §M34's "record, port, then change". From here
# its implementation is `crates/screen`. M34 P5 ports it faithfully against these files, and P10
# re-records them with the shipping Rust parser through `golf-core rerecord`, after which frozen
# Python never writes them again. **So this recorder runs once.** `conformance.py regenerate
# --screen-once` refuses as soon as any screen vector is committed, on `_audio`'s precedent, and
# these functions stay here and stay runnable (docs/README.md §Conventions) as the record of how
# the family was made.
#
# Four sub-families, each covering what the others cannot:
#
# - `corpus/`: PaddleOCR's boxes for every distinct shot photo the bundles carry, and frozen
#   Python's parse of each. Real OCR noise, real geometry, the real `Impact Position` pair. Each is
#   **verified against the shot the store already holds** before anything is written, because
#   M34's planning run measured today's OCR reproducing every stored shot exactly; a vector that
#   differs is not the shot on disk.
# - `reference/`: the two photos of the other layout (`Bounce & Roll`, no V tile), read directly
#   and stored nowhere.
# - `synthetic/`: `tests/launch_monitor/conftest.py::build_screen`, so the screens the Python tests
#   trust are the ones the port is gated on, one vector per path the parser and validator tests
#   take.
# - `units/`: the parser's private functions over crafted tables, for edges no screen reaches often
#   enough to be gated by one.
#
# **The documents carry floats as JSON numbers, and the tables carry them as `repr`.** A screen
# vector is shaped like an engine vector, an `input` a port parses into its own types
# (`screen::TextBox` derives `Deserialize`), and the workspace's `serde_json` has
# `float_roundtrip`, so a JSON number lands on Python's bits. A `units` table is shaped like the
# format family's and keeps that family's convention, because `_first_number` can answer `-0.0`
# and a reader should see the sign in the file.
#
# **The version is `screen_parser_version`, top-level, where every family keeps the one it ages
# on** (`analysis_version`, `detector_version`, `python_version`) and where `golf-core rerecord`
# moves `analysis_version`. The M34 plan's call 1 listed it under `provenance`; top-level is where
# `conformance.py list` and the re-record's version move both look. Frozen Python's parser has no
# version. `crates/contracts/src/shot.rs::SCREEN_PARSER_VERSION`'s ledger reads 0 as "unstamped,
# frozen Python's", so 0 is what these files say until P10 moves them to Rust's.

#: Frozen Python's place in `SCREEN_PARSER_VERSION`'s ledger. See the section header.
_FROZEN_SCREEN_PARSER = 0

#: The two photos of the reference layout, in `data/raw/` (gitignored, like every capture).
_REFERENCE_PHOTOS = REPO / "data" / "raw" / "shot_screens"

#: The timestamp a vector gives a shot no bundle owns. Fixed, so a re-run writes the same bytes; it
#: is the date `test_to_shot_data_carries_the_parse_provenance` stamps.
_FIXED_TIMESTAMP = "2026-08-04T12:00:00Z"


def build_screen() -> list[tuple[Path, dict[str, Any]]]:
    """The screen family, as (path, payload) pairs, every one built and verified before any is
    returned, so a photo that fails its check leaves `spec/` untouched rather than half-written.

    Needs the `ocr` extra (PaddleOCR, OpenCV, and pillow-heif for the HEIC photos) and `data/`,
    which is why `regenerate --screen-once` is the only caller and `check` never imports it. The
    recognizer and the profile are the ones `api/pipeline.py::_shot_for` builds, from the same
    settings, because the corpus half is checked against what that function stored.
    """
    from golf_coach.config import settings
    from golf_coach.launch_monitor.screen.importer import build_recognizer

    recognizer = build_recognizer(settings.ocr_engine)
    profile = load_profile(settings.launch_monitor_profile)
    families = {
        "corpus": _screen_corpus(recognizer, profile),
        "reference": _screen_reference(recognizer, profile),
        "synthetic": _screen_synthetic(),
        "units": _screen_units(),
    }
    return [
        (VECTORS / "screen" / family / f"{name}.json", vector)
        for family, built in families.items()
        for name, vector in built.items()
    ]


def _screen_corpus(recognizer: Any, profile: Any) -> dict[str, dict[str, Any]]:
    """Every distinct shot photo the bundles carry, found and identified the way the pipeline does.

    **Through the bundles, never through `import_shot_screens.py`'s bulk path** (the program plan's
    M31.5 errata). Each manifest's `shot_screen` role names the photo and its sha256, and the stored
    shot is the file `ShotStore.get(sha256)` reads, which is `api/pipeline.py::_shot_for`'s lookup
    and the one that attached these shots to their swings. Deduplicated on that sha, because one
    photo can be the shot screen of several bundles and still has one stored shot, and a vector per
    bundle would be several files recording one parse. Each is named for its stored `shot_id`.
    """
    from golf_coach.config import settings
    from golf_coach.launch_monitor.screen.store import ShotStore, hash_image
    from golf_coach.storage.manifest import Role, SwingManifest

    store = ShotStore(settings.shots_dir)
    bundles: dict[str, list[Path]] = {}
    for manifest_path in sorted(SESSIONS.glob("*/*/manifest.json")):
        manifest = SwingManifest.model_validate_json(manifest_path.read_text(encoding="utf-8"))
        role_file = manifest.roles.get(Role.SHOT_SCREEN)
        if role_file is not None:
            photo = manifest_path.parent / role_file.filename
            bundles.setdefault(role_file.content_sha256, []).append(photo)

    built: dict[str, dict[str, Any]] = {}
    for digest, photos in bundles.items():
        stored_path = store.path_for(digest)
        if not stored_path.exists():
            raise AssertionError(
                f"{_rel(photos[0])}: no stored shot for sha256 {digest} — the corpus is the shots "
                f"the store holds, so there is nothing to verify this photo's parse against"
            )
        # The raw file rather than `store.get`'s model: the timestamp travels as the string the
        # store wrote, and the verify below compares the JSON the store holds, not a re-dump of it.
        stored = json.loads(stored_path.read_text(encoding="utf-8"))
        photo = _stored_photo(stored, photos)
        if hash_image(photo.read_bytes()) != digest:
            raise AssertionError(f"{_rel(photo)} does not hash to its manifest's sha256 {digest}")

        name = stored["shot_id"]
        assert name not in built, f"two photos are stored as shot {name}"
        given, label_ratio = _photo_input(
            photo,
            recognizer,
            profile,
            shot_id=stored["shot_id"],
            session_id=stored["session_id"],
            timestamp=stored["timestamp"],
            digest=digest,
        )
        bundle_dirs = ", ".join(_rel(p.parent) for p in photos)
        vector = _screen_vector(
            f"screen/corpus/{name}",
            given,
            note=(
                "a stored bay photo, PaddleOCR's boxes and frozen Python's parse of them, verified "
                "equal to the stored shot on every key but provenance.image_path"
                + _duplicate_labels(given, profile)
            ),
            source=f"{_rel(photo)} (shot screen of {bundle_dirs}); stored shot {_rel(stored_path)}",
            extra=_ocr_provenance(),
        )
        _verify_screen_against_stored(vector, stored, label_ratio, name)
        built[name] = vector
    return built


def _stored_photo(stored: dict[str, Any], photos: list[Path]) -> Path:
    """The bundle copy the stored shot names, or the first: the copies hash identically, and the
    vector's `image_path` should be the one the store already points at where it can be."""
    recorded = (stored.get("provenance") or {}).get("image_path")
    if recorded:
        for photo in photos:
            if photo.resolve() == Path(recorded).resolve():
                return photo
    return photos[0]


def _screen_reference(recognizer: Any, profile: Any) -> dict[str, dict[str, Any]]:
    """The two reference photos: the layout with `Bounce & Roll` and no V tile, stored nowhere."""
    from golf_coach.launch_monitor.screen.store import hash_image

    built: dict[str, dict[str, Any]] = {}
    for photo in sorted(_REFERENCE_PHOTOS.glob("IMG_*.jpeg")):
        given, label_ratio = _photo_input(
            photo,
            recognizer,
            profile,
            shot_id=photo.stem,
            session_id="reference",
            timestamp=_FIXED_TIMESTAMP,
            digest=hash_image(photo.read_bytes()),
        )
        vector = _screen_vector(
            f"screen/reference/{photo.stem}",
            given,
            note=(
                "a reference photo of the other layout, read directly and written to no store; "
                "the identity fields are fixed, the sha256 and path are the photo's"
                + _duplicate_labels(given, profile)
            ),
            source=_rel(photo),
            extra=_ocr_provenance(),
        )
        assert vector["expected"]["label_ratio"] == label_ratio, f"{photo.name}: label_ratio moved"
        built[photo.stem] = vector
    assert len(built) == 2, f"expected the two reference photos in {_rel(_REFERENCE_PHOTOS)}"
    return built


def _photo_input(
    photo: Path,
    recognizer: Any,
    profile: Any,
    *,
    shot_id: str,
    session_id: str,
    timestamp: str,
    digest: str,
) -> tuple[dict[str, Any], float]:
    """A photo through `prepare_screen`, as a screen vector's `input`, with the `label_ratio` the
    rotation vote settled on so the caller can hold `expected` to it.

    `prepare_screen` is `import_screen`'s own call, so the boxes and the notes are exactly what the
    parser was handed when the shot was stored. The notes travel beside the boxes because
    `import_screen` prepends them to the warnings before validating (planning finding 8).
    `image_path` is repo-relative with `/`: the store holds absolute Windows paths, and a vector
    must read the same on every machine.
    """
    from dataclasses import asdict

    from golf_coach.config import settings
    from golf_coach.launch_monitor.screen.preprocess import load_image, prepare_screen

    print(f"  screen   reading {_rel(photo)} ...", file=sys.stderr, flush=True)
    prepared = prepare_screen(load_image(photo), recognizer, profile)
    given = {
        "device": profile.device,
        "boxes": [asdict(box) for box in prepared.boxes],
        "notes": list(prepared.notes),
        "shot_id": shot_id,
        "session_id": session_id,
        "timestamp": timestamp,
        "image_sha256": digest,
        "image_path": _rel(photo),
        "min_confidence": settings.ocr_min_confidence,
    }
    return given, prepared.label_ratio


def _ocr_provenance() -> dict[str, Any]:
    """Which OCR produced a photo vector's boxes. The boxes are input, so this ages nothing; it
    says what to install to read the same photo the same way."""
    import paddleocr

    return {"paddleocr_version": str(paddleocr.__version__)}


def _rel(path: Path) -> str:
    return path.relative_to(REPO).as_posix()


def _duplicate_labels(given: dict[str, Any], profile: Any) -> str:
    """Name every label more than one box matched, for a photo vector's note.

    Measured from the boxes rather than written by hand, because which photos carry the
    `Impact Position` pair is what M34's tie rule changes, and a note that guessed would be the
    first thing wrong in the file. `_find_labels` keeps the first box with the best score (`>`).
    """
    matches: dict[str, list[str]] = {}
    for box in given["boxes"]:
        field = profile.field_for(box["text"])
        if field is not None:
            score = field.matches(box["text"])
            matches.setdefault(field.label, []).append(
                f"{box['text']!r} at x={box['x']:.0f} scores {score:.4f}"
            )
    pairs = [
        f"{label!r} by {', '.join(found)}" for label, found in matches.items() if len(found) > 1
    ]
    if not pairs:
        return ""
    return (
        ". More than one box matches "
        + "; ".join(pairs)
        + " (`_find_labels` keeps the first box with the best score)"
    )


def _screen_vector(
    vector_id: str,
    given: dict[str, Any],
    *,
    note: str,
    source: str,
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Serialize the input, run it through `_run_screen`, and pair the two.

    Serialized **before** it is run, `_vector`'s rule: what gets recorded is what a reader parses
    back, not the live objects that built it.
    """
    given_json = json.loads(json.dumps(given))
    return {
        "id": vector_id,
        "screen_parser_version": _FROZEN_SCREEN_PARSER,
        "provenance": {
            "kind": "screen",
            "oracle": "python",
            "note": note,
            "source": source,
            "python_version": _python_version(),
            **(extra or {}),
        },
        "input": given_json,
        "expected": _run_screen(given_json),
    }


def _run_screen(given: dict[str, Any]) -> dict[str, Any]:
    """One screen vector's input through frozen Python: the definition a port reproduces.

    `import_screen` from its `prepare_screen` call on, which is the half that does not touch
    pixels: `parse_screen`, the notes prepended, `validate_parse`, then `to_shot_data` where the
    read did not fail. `expected.parsed` is the parse **before** validation (confidence unrounded,
    warnings without the notes) and `expected.shot` the record after it, so a port can be gated on
    its parser before it has a validator (the M34 plan's call 1). `shot` is `None` exactly where
    `import_screen` returns `failed`.
    """
    from dataclasses import replace
    from datetime import datetime

    from golf_coach.launch_monitor.screen.parser import parse_screen, to_shot_data
    from golf_coach.launch_monitor.screen.preprocess import _label_ratio
    from golf_coach.launch_monitor.screen.recognizer import TextBox
    from golf_coach.launch_monitor.screen.validate import validate_parse

    profile = load_profile(given["device"])
    boxes = [TextBox(**box) for box in given["boxes"]]
    parsed = parse_screen(boxes, profile)
    expected: dict[str, Any] = {
        "label_ratio": _label_ratio(boxes, profile),
        "parsed": {
            "values": dict(parsed.values),
            "raw_fields": dict(parsed.raw_fields),
            "confidence": parsed.confidence,
            "warnings": list(parsed.warnings),
        },
    }
    # `import_screen`'s `parsed.warnings[:0] = prepared.notes`, on a copy so `expected.parsed`
    # above keeps the parser's own list.
    noted = replace(parsed, warnings=[*given["notes"], *parsed.warnings])
    validated = validate_parse(noted, min_confidence=given["min_confidence"])
    shot = None
    if not validated.is_empty:
        record = to_shot_data(
            validated,
            shot_id=given["shot_id"],
            session_id=given["session_id"],
            timestamp=datetime.fromisoformat(given["timestamp"]),
            image_sha256=given["image_sha256"],
            image_path=given["image_path"],
        )
        shot = json.loads(record.model_dump_json())
    expected["shot"] = shot
    return expected


def _verify_screen_against_stored(
    vector: dict[str, Any], stored: dict[str, Any], label_ratio: float, name: str
) -> None:
    """Assert the vector reproduces the stored shot exactly, on every key but `image_path`.

    `_verify_against_stored`'s job for the screen family. Without it these are files that agree
    with whatever today's OCR read, which is evidence of nothing; with it they are the shots on
    disk. Equality and not `compare_results`' tolerance: the stored file is this same parse
    serialized by this same pydantic, so any difference at all means the OCR read something else
    and the boxes are not the ones that produced the stored shot. `image_path` is the one key
    allowed to differ, because the store holds an absolute Windows path and the vector a
    repo-relative one.
    """
    got = copy.deepcopy(vector["expected"]["shot"])
    if got is None:
        raise AssertionError(
            f"{name}: frozen Python failed to read a photo the store holds a shot for"
        )
    want = copy.deepcopy(stored)
    for document in (got, want):
        document["provenance"].pop("image_path", None)
    if got != want:
        from conformance import compare_results

        diffs = compare_results(want, got, "shot") or ["(equal under tolerance: float bits differ)"]
        raise AssertionError(
            f"{name}: today's OCR and parse do not reproduce the stored shot — "
            + "; ".join(str(d) for d in diffs[:5])
        )
    if vector["expected"]["label_ratio"] != label_ratio:
        raise AssertionError(f"{name}: label_ratio is not the one the rotation vote settled on")


# ------------------------------------------------------------------------ screen: synthetic


def _screen_synthetic() -> dict[str, dict[str, Any]]:
    """`build_screen`'s screens, one per path the parser and validator tests take.

    Each case names the test it mirrors, and the rows are edited the way that test edits them, so
    when a Python test and a vector disagree about a screen it is one screen. Two are not in any
    test, and their notes say why they are here. The identity is fixed and there is no photo, so
    `image_sha256` and `image_path` are `None`, which is the path `to_shot_data` takes for a shot
    with no image.
    """
    from golf_coach.config import settings
    from golf_coach.launch_monitor.screen.recognizer import TextBox

    fixtures = _load_conftest(
        REPO / "tests" / "launch_monitor" / "conftest.py", "_conformance_screen_conftest"
    )
    build = fixtures.build_screen
    first, second = fixtures.SCREEN_2738, fixtures.SCREEN_2739

    def edited(*edits: tuple[int, int, str, list[str]]) -> list[Any]:
        rows = [list(first[0]), list(first[1])]
        for row, column, label, lines in edits:
            rows[row][column] = (label, lines)
        return rows

    split = build(edited((0, 6, "Club Path", ["1.6"])))
    number = next(b for b in split if b.text == "1.6")
    split.append(TextBox("° O>I", number.right + 4, number.y, 40.0, number.height, 0.9))

    # A stray `Carry` above the grid, seen *before* the real one: both score 1.0, so `_find_labels`
    # keeps the stray, and the real label box is left as ordinary text.
    stray = build(first)
    stray.insert(1, TextBox("Carry", 300.0, 40.0, 35.0, 20.0, 0.95))

    tests = "tests/launch_monitor/test_screen_parser.py::"
    checks = "tests/launch_monitor/test_screen_validate.py::"
    minimum = settings.ocr_min_confidence
    cases: list[tuple[str, str, str, list[Any], float]] = [
        (
            "reference-2738",
            "IMG_2738 as the fixtures transcribe it: every tile read, `---` read as None and never "
            "0, a boundary-only tile with nothing under it, `O>I`, `Closed` and `L` setting signs, "
            "and the title matched without its spaces (`SHOTDATA`). The case a port gets working "
            "first.",
            f"{tests}test_reads_every_tile_of_a_real_screen, "
            "test_blank_tiles_become_none_not_zero, "
            f"test_the_title_is_matched_without_its_spaces; {checks}"
            "test_a_clean_screen_passes_untouched",
            build(first),
            minimum,
        ),
        (
            "reference-2739",
            "IMG_2739: the opposite signs (`I>O`, `R`), and a 0.89 smash factor that is odd but "
            "self-consistent, so it passes validation.",
            f"{tests}test_reads_the_opposite_signs_on_the_second_screen; {checks}"
            "test_an_implausible_but_self_consistent_shot_still_passes",
            build(second),
            minimum,
        ),
        (
            "split-value-boxes",
            "`1.6` and `° O>I` as two boxes on one line, at a lower confidence than the rest: the "
            "cell rejoins them left to right, and the sign still comes from the word.",
            f"{tests}test_split_value_boxes_are_rejoined_in_reading_order",
            split,
            minimum,
        ),
        (
            "missing-direction-word",
            "`1.6 °` with no qualifier: the magnitude is stored as printed and a warning says the "
            "sign is unknown.",
            f"{tests}test_missing_direction_word_is_reported_not_guessed",
            build(edited((0, 6, "Club Path", ["1.6 °"]))),
            minimum,
        ),
        (
            "printed-sign",
            "`-9.3 °` in the Spin Axis tile: a printed sign the profile's `printed_sign: -1` "
            "inverts to +9.3, so the parser warns about nothing. The validator then flags +9.3 (a "
            "fade) against the fixture's SLIGHT DRAW, which is the shape check doing its job.",
            f"{tests}test_a_printed_sign_is_a_known_sign_not_a_missing_word",
            build(edited((1, 6, "Spin Axis", ["-9.3 °"]))),
            minimum,
        ),
        (
            "word-beats-printed-sign",
            "`-9.3 ° R`: a direction word and a printed sign both, and the word wins (+9.3, which "
            "the validator flags against SLIGHT DRAW).",
            f"{tests}test_a_direction_word_still_beats_a_printed_sign",
            build(edited((1, 6, "Spin Axis", ["-9.3 ° R"]))),
            minimum,
        ),
        (
            "unsigned-no-word",
            "`9.3 °` in a `printed_sign` tile: no sign and no word, so still reported as unknown.",
            f"{tests}test_an_unsigned_number_with_no_word_is_still_reported",
            build(edited((1, 6, "Spin Axis", ["9.3 °"]))),
            minimum,
        ),
        (
            "title-cropped",
            "No box above the grid at all: the title's absence is a fact about the crop, so it is "
            "not warned about. The test's filter drops every box containing `SHOT`, so the Shot "
            "Distance and Shot Type labels go too, their neighbours' columns widen, and Horizontal "
            "Angle reads `SLIGHT 2.6 ° L DRAW`, a residual no sign rule matches.",
            f"{tests}test_a_title_cropped_out_of_frame_is_not_reported_as_missing",
            [b for b in build(first) if "SHOT" not in b.text.upper()],
            minimum,
        ),
        (
            "title-missing",
            "Text above the grid that is not the title: the one case the title warning is for.",
            f"{tests}test_a_missing_title_is_reported_when_we_could_have_seen_it",
            build(first, title="PUTTING ANALYSIS"),
            minimum,
        ),
        (
            "unreadable",
            "One box and no label: no values, confidence 0, and `shot` is None because "
            "`import_screen` returns `failed` rather than a half-populated shot.",
            f"{tests}test_unreadable_screen_yields_nothing_rather_than_garbage",
            [TextBox("BALL TRAJECTORY", 0.0, 0.0, 200.0, 20.0, 0.9)],
            minimum,
        ),
        (
            "partial",
            "The top row alone: half the labels, a warning per missing tile, and a confidence "
            "that clears the threshold.",
            f"{tests}test_partial_screen_lowers_confidence",
            build([first[0]]),
            minimum,
        ),
        (
            "partial-below-min-confidence",
            "The same top row under a 0.9 threshold: no cross-check fails, and the review flag is "
            "set by the confidence alone.",
            f"{checks}test_low_confidence_alone_flags_a_review",
            build([first[0]]),
            0.9,
        ),
        (
            "spin-axis-disagrees",
            "FADE beside a left-tilted axis: every magnitude right and the sign wrong, which only "
            "the shape word can catch.",
            f"{checks}test_a_fade_stored_as_a_draw_is_caught_by_the_shape_word",
            build(edited((1, 4, "Shot Type", ["FADE"]), (1, 6, "Spin Axis", ["9.3 ° L"]))),
            minimum,
        ),
        (
            "spin-axis-agrees",
            "FADE beside a printed `-9.3 °`, stored +9.3: the axis and the word agree, silently.",
            f"{checks}test_the_axis_and_the_shape_word_agreeing_is_silent",
            build(edited((1, 4, "Shot Type", ["FADE"]), (1, 6, "Spin Axis", ["-9.3 °"]))),
            minimum,
        ),
        (
            "near-zero-axis",
            "SLIGHT FADE beside 0.4 ° L: inside the deadband, so the word is not evidence.",
            f"{checks}test_a_near_zero_axis_is_not_judged_against_the_shape_word",
            build(
                edited((1, 4, "Shot Type", ["SLIGHT", "FADE"]), (1, 6, "Spin Axis", ["0.4 ° L"]))
            ),
            minimum,
        ),
        (
            "straight-shot",
            "STRAIGHT names no curvature, so the axis has nothing to disagree with.",
            f"{checks}test_a_straight_shot_gives_the_axis_nothing_to_disagree_with",
            build(edited((1, 4, "Shot Type", ["STRAIGHT"]), (1, 6, "Spin Axis", ["-9.3 °"]))),
            minimum,
        ),
        (
            "distance-identity",
            "Carry 128.1 read as 28.1: carry + bounce & roll no longer makes the shot distance, "
            "and the message carries three `:g` numbers.",
            f"{checks}test_a_dropped_digit_in_carry_breaks_the_distance_identity",
            build(edited((0, 1, "Carry", ["28.1", "yds"]))),
            minimum,
        ),
        (
            "smash-identity",
            "Ball speed 101.5 read as 10.5: the smash identity breaks, and the message carries a "
            "`.3f` quotient.",
            f"{checks}test_a_misread_speed_breaks_the_smash_identity",
            build(edited((0, 3, "Ball Speed", ["10.5", "mph"]))),
            minimum,
        ),
        (
            "range",
            "Ball speed 1015: outside its plausible range, and the smash identity breaks too, so "
            "both penalties land.",
            f"{checks}test_an_inserted_digit_is_caught_by_range",
            build(edited((0, 3, "Ball Speed", ["1015", "mph"]))),
            minimum,
        ),
        (
            "duplicate-label",
            "In no test. A stray `Carry` above the grid, seen before the real one: both score 1.0 "
            "and `_find_labels` keeps the first box (`>`), so Carry is read under the stray, and "
            "the real label box is left as ordinary text. This is how a tie is resolved before "
            "M34's tie rule, and the faithful port has to reproduce it.",
            "src/golf_coach/launch_monitor/screen/parser.py::_find_labels",
            stray,
            minimum,
        ),
    ]

    built: dict[str, dict[str, Any]] = {}
    for name, note, source, boxes, min_confidence in cases:
        given = {
            "device": "hd_golf",
            "boxes": [_box(b) for b in boxes],
            "notes": [],
            "shot_id": f"synthetic-{name}",
            "session_id": "synthetic",
            "timestamp": _FIXED_TIMESTAMP,
            "image_sha256": None,
            "image_path": None,
            "min_confidence": min_confidence,
        }
        built[name] = _screen_vector(
            f"screen/synthetic/{name}",
            given,
            note=note,
            source=f"tests/launch_monitor/conftest.py::build_screen; {source}",
        )
    return built


def _box(box: Any) -> dict[str, Any]:
    from dataclasses import asdict

    return asdict(box)


# ---------------------------------------------------------------------------- screen: units
#
# The parser's private functions, each over a crafted table, recorded with the family because the
# answers are the frozen *parser's*, not the language's (the format family has those). Every case
# carries the profile it ran against, as data, so the V tile the Rust fork gains in P8 cannot move
# an answer recorded here: `_sign_from` and `_is_blank` cases carry their field and blank markers
# inline, and `field_for` cases name one of the table's own `profiles`.

#: Texts no photo produced, for the label matcher: OCR damage the threshold is argued from, the
#: punctuation `normalize_label` turns to spaces, and letters `str.upper` moves out of `A-Z`.
_UNITS_LABEL_TEXTS: tuple[str, ...] = (
    "Carny",
    "Bounce &Roll",
    "Bounce Rol",
    "lmpact Position",
    "Impact Position Y",
    "Club Peed",
    "  Club--Path  ",
    "spin_axis",
    "Shot\tDistance",
    "CARRY!",
    "carry",
    "\uff23\uff41\uff52\uff52\uff59",
    "\uff23arry",
    "Spin Axis 2",
    "Spinn",
)


def _screen_units() -> dict[str, dict[str, Any]]:
    from golf_coach.launch_monitor.screen import parser
    from golf_coach.launch_monitor.screen.profiles import DeviceProfile, ProfileField
    from golf_coach.launch_monitor.screen.recognizer import TextBox

    def tb(text: str, x: float, y: float, width: float, height: float) -> TextBox:
        return TextBox(text, float(x), float(y), float(width), float(height))

    profile = load_profile("hd_golf")
    by_label = {field.label: field for field in profile.fields}
    texts = _unique([*_OCR_TEXTS, *_profile_texts(), *_UNITS_LABEL_TEXTS, *_CRAFTED_TEXTS])

    # --- normalize_label
    normalize = [{"text": t, "expected": normalize_label(t)} for t in texts]

    # --- field_for, with `>=`: a later field wins at an equal score, and the threshold is met at
    # equality. `hd_golf` has no two labels a real text ties on, so `tie` is two fields any
    # `Spin ?` scores identically against: 5/6 each for `Spin Z`, and exactly the 0.8 threshold
    # each for a bare `Spin`.
    tie = DeviceProfile.model_validate(
        {
            "device": "tie",
            "title": "TIE",
            "fields": [
                {"label": "Spin X", "target": "x"},
                {"label": "Spin Y", "target": "y"},
                {"label": "Boundary", "target": None},
            ],
        }
    )
    profiles = {"hd_golf": profile, "tie": tie}
    field_for: list[dict[str, Any]] = []
    for profile_name, texts_for in (
        ("hd_golf", texts),
        ("tie", ["Spin Z", "Spin", "Spin X", "Spin Y", "SPIN-Y", "Spi", "Boundary", "Bound", ""]),
    ):
        for text in texts_for:
            matched = profiles[profile_name].field_for(text)
            field_for.append(
                {
                    "profile": profile_name,
                    "text": text,
                    "label": None if matched is None else matched.label,
                    "score": None if matched is None else repr(matched.matches(text)),
                }
            )

    # --- _first_number, with `_THOUSANDS`' look-behind and `\d{3}\b` look-ahead. `\d` is
    # Unicode-aware on a `str` pattern and `float()` reads any decimal digit, so a Unicode digit is
    # a number here. `_THOUSANDS` runs over the *matched* text, not the cell, so its `\b` only ever
    # sees the end of the match, a `,`, a `.` or a fourth digit after the three: `5,991rpm` loses
    # its comma exactly as `5,991` does, which the `_`, `é` and `\x1c` cases are here to show.
    numbers = _unique(
        [
            *(t for t in _OCR_TEXTS if any(c.isdigit() for c in t)),
            "", "---", "abc", "128.1", "128.1 yds", "+3.0", "-0.0", "0.0\xb0L", ".5", "-.5",
            "- 5", "--5", "+-5", "1.", "1.2.3", "1e5",
            # Thousands at the `\d{3}\b` boundary and off it.
            "5,991", "5,991 rpm", "5,991rpm", "5,991_", "5,991\xe9", "5,991\xb0", "5,991\x1c",
            "1,234,567", "1,234,567.5", "12,345.6", "1,234,56", "1,000,000",
            # Commas left as decimal points, and the two that `float()` then refuses.
            "12,34", "1,2345", "1,2,3", "1,2.5",
            # Unicode digits: Arabic-Indic, fullwidth, mixed, grouped. A superscript is not `\d`.
            "\u0661\u0662\u0663", "\uff11\uff12.\uff15", "1\u06623", "\u0663,\u0664\u0665\u0666",
            "\xb2", "x\xb2 = 4",
        ]
    )
    first_number = []
    for text in numbers:
        found = parser._first_number(text)
        first_number.append(
            {
                "text": text,
                "expected": None
                if found is None
                else {"value": repr(found[0]), "matched": found[1]},
            }
        )

    # --- _sign_from: the residual after the number is taken out, against the field's rules. Rules
    # in order and the first hit wins; a one-letter token must be the whole residual, so `L` never
    # matches inside `CLOSED`; a token that sanitizes to nothing is skipped.
    crafted_field = ProfileField(
        label="Crafted",
        target="crafted",
        sign_tokens=[
            {"tokens": ["\xb0", "X"], "sign": 1},
            {"tokens": ["-", "Y"], "sign": -1},
        ],
    )
    residuals: list[tuple[Any, list[str]]] = [
        (
            by_label["Club Path"],
            [" \xb0 O>I", " \xb0 I>O", "\xb00>1", "\xb01>0", "\xb00>I", " O I", "IO", " \xb0 o>i",
             " \xb0 O>I I>O", " \xb0 ", "", " \xb0 OI!"],
        ),
        (
            by_label["Club Face Angle"],
            [" \xb0 Closed", " Open", " \xb0 Close", "\xb0CLOSED", " \xb0 Reopen",
             " \xb0 Open\u017f", " \xb0 OPEN CLOSED", " \xb0 cl\u014dsed", " \xb0 Opened"],
        ),
        (
            by_label["Horizontal Angle"],
            [" \xb0 L", "\xb0L", " \xb0 R", " \xb0 Left", " \xb0 RIGHT", " \xb0 LR", " \xb0 CLOSED",
             " l", " \xb0 L.", " \xb0 RL"],
        ),
        (by_label["Spin Axis"], [" \xb0", " \xb0 R", "\xb0 L", "-"]),
        (crafted_field, [" \xb0 ", " X", " x", " Y", "XY", " - "]),
    ]
    sign_from = [
        {
            "residual": residual,
            "field": field.model_dump(mode="json"),
            "expected": parser._sign_from(residual, field),
        }
        for field, cases in residuals
        for residual in cases
    ]

    # --- _is_blank: every marker spaced and unspaced, against `hd_golf`'s markers, pydantic's
    # default, and two that compact to nothing under `\s+` (which then blanks an empty cell).
    marker_sets = [
        list(profile.blank_markers),
        DeviceProfile(device="default", title="", fields=[]).blank_markers,
        [" "],
        ["\x1c-\x1c"],
    ]
    blank_texts = _unique(
        [
            *profile.blank_markers,
            *(" " + m + " " for m in profile.blank_markers),
            *(m.replace(" ", "") for m in profile.blank_markers),
            "-", "----", "-\xa0-\xa0-", "-\x1c-\x1d-", "---\n", "\t---\t", "\u2013", "\u2014 ",
            "\u3000-\u3000-\u3000-", "", " ", "\x1c", "N/A", "0", "--- yds", "-",
        ]
    )
    is_blank = [
        {
            "text": text,
            "blank_markers": markers,
            "expected": parser._is_blank(
                text, profile.model_copy(update={"blank_markers": markers})
            ),
        }
        for markers in marker_sets
        for text in blank_texts
    ]

    # --- _Cell.text: lines keyed `int(center_y // bucket)`, then `center_x`, stably; boxes
    # stripped with Python's whitespace and dropped when nothing is left.
    field = profile.fields[0]
    cells: list[tuple[str, TextBox, list[TextBox]]] = [
        ("one box", tb("L", 0, 100, 80, 20), [tb("128.1", 40, 135, 45, 24)]),
        (
            "one line, given right to left",
            tb("L", 0, 100, 80, 20),
            [tb("O>I", 90, 135, 30, 24), tb("1.6 \xb0", 40, 135, 45, 24)],
        ),
        (
            "two lines, given bottom first",
            tb("L", 0, 100, 80, 20),
            [tb("yds", 50, 165, 27, 24), tb("128.1", 40, 135, 45, 24)],
        ),
        (
            # A 7 px label makes the bucket 4.2, and `21.0 // 4.2` is 4.0 where `21.0 / 4.2`
            # rounds to exactly 5.0, so `floor(a / b)` puts both boxes on line 5 and reads them by
            # x as `B A`. CPython keeps `A` on line 4 and reads `A B`.
            "floor division, not floor of a division",
            tb("L", 0, 0, 80, 7),
            [tb("A", 95, 17, 10, 8), tb("B", 45, 17.5, 10, 8)],
        ),
        (
            "equal keys keep the order given",
            tb("L", 0, 100, 80, 20),
            [tb("X", 40, 135, 20, 24), tb("Y", 40, 135, 20, 24)],
        ),
        (
            "equal keys keep the order given, the other way",
            tb("L", 0, 100, 80, 20),
            [tb("Y", 40, 135, 20, 24), tb("X", 40, 135, 20, 24)],
        ),
        (
            # `str.strip()` takes the four C0 separators and NBSP; `str::trim` keeps the first four.
            "Python's whitespace stripped, and blank boxes dropped",
            tb("L", 0, 100, 80, 20),
            [
                tb("\x1c1.6\x1d", 40, 135, 30, 24),
                tb(" ", 75, 135, 5, 24),
                tb("\u3000", 82, 135, 5, 24),
                tb("\x1c", 88, 135, 5, 24),
                tb("\xb0\xa0", 95, 135, 10, 24),
            ],
        ),
        (
            # Floor, not truncation: a center above zero's bucket is line -1, not line 0.
            "a negative center floors down",
            tb("L", 0, 0, 80, 20),
            [tb("down", 5, 0, 10, 8), tb("up", 45, -10, 10, 8)],
        ),
        (
            "a zero-height label buckets by the 1e-6 floor",
            tb("L", 0, 100, 80, 0),
            [tb("b", 0, 101, 10, 0), tb("a", 50, 100, 10, 0)],
        ),
        ("no boxes", tb("L", 0, 100, 80, 20), []),
    ]
    cell_text = [
        {
            "case": case,
            "label_box": _box(label_box),
            "value_boxes": [_box(b) for b in value_boxes],
            "expected": parser._Cell(field, label_box, tuple(value_boxes)).text,
        }
        for case, label_box, value_boxes in cells
    ]

    screen = "src/golf_coach/launch_monitor/screen/"
    return {
        "normalize_label": _units_vector(
            "normalize_label",
            f"{screen}profiles.py::normalize_label",
            "`normalize_label`: upper-cased, everything outside `A-Z0-9` a space, stripped — over "
            "every box text of the planning run, the profile's strings, OCR damage and crafted "
            "Unicode",
            normalize,
        ),
        "field_for": _units_vector(
            "field_for",
            f"{screen}profiles.py::DeviceProfile.field_for",
            "`field_for(text)` and the winner's `matches(text)`, threshold met at equality and the "
            "later field winning a tie (`>=`), against the frozen `hd_golf` profile and a crafted "
            "`tie` profile; each case names its profile in `profiles`",
            field_for,
            profiles={name: p.model_dump(mode="json") for name, p in profiles.items()},
        ),
        "first_number": _units_vector(
            "first_number",
            f"{screen}parser.py::_first_number",
            "`_first_number`: the first `[-+]?\\d[\\d,]*(?:\\.\\d+)?`, `,` dropped before three "
            "digits and a word boundary and read as `.` otherwise, then `float()`; `None` where "
            "nothing matches or `float()` refuses. `value` is a `repr`",
            first_number,
        ),
        "sign_from": _units_vector(
            "sign_from",
            f"{screen}parser.py::_sign_from",
            "`_sign_from(residual, field)`: the residual upper-cased and cut to `A-Z0-9>`, against "
            "each rule's tokens in order; one-letter tokens must be the whole residual. Each case "
            "carries its field",
            sign_from,
        ),
        "is_blank": _units_vector(
            "is_blank",
            f"{screen}parser.py::_is_blank",
            "`_is_blank(text, profile)`: `\\s+` removed from both sides, then equality with any "
            "marker. Each case carries the profile's `blank_markers`",
            is_blank,
        ),
        "cell_text": _units_vector(
            "cell_text",
            f"{screen}parser.py::_Cell.text",
            "`_Cell.text`: value boxes ordered by `(int(center_y // bucket), center_x)` with "
            "`bucket = max(label height * 0.6, 1e-6)`, stably, each `strip()`ped and dropped if "
            "empty, joined by a space",
            cell_text,
        ),
    }


def _units_vector(
    name: str, source: str, note: str, cases: list[dict[str, Any]], **extra: Any
) -> dict[str, Any]:
    assert cases, f"screen/units/{name} recorded no cases"
    return {
        "id": f"screen/units/{name}",
        "screen_parser_version": _FROZEN_SCREEN_PARSER,
        "provenance": {
            "kind": "screen",
            "oracle": "python",
            "note": note,
            "source": source,
            "python_version": _python_version(),
        },
        "cases": cases,
        **extra,
    }


# ===================================================================== storage: the corpus reader
#
# **The storage family records the many-shot layer's stores and the reader over them** (M36, the
# M36 plan's call 1). This half, from P1, exercises `storage/corpus.py::read_corpus` and
# `narrow_to`; the store operations are the next section's (P2). `regenerate --storage-once`
# records the whole family once, through `build_storage`, and refuses a second run, on the screen
# family's precedent. From there `crates/storage` is the implementation and `golf-core rerecord`
# the only writer.
#
# **A case's input is the sessions directory as raw text.** `input.files` maps a path relative to
# the sessions root to that file's text, so a corrupt manifest or a half-written state file is a
# case like any other. `_materialise` is how a port lays it out: a key ending in `/` is an empty
# directory, and `files: null` is a root that does not exist. Frozen Python answers a missing root
# as it answers an empty one, but a port's directory read meets two different things. Only what
# `read_corpus` opens travels (`manifest.json`, `analysis.json`, `analysis.state.json`), plus
# `session.json`, because a session directory holding only that is still a session scanned.
#
# **The synthetic trees are written by `tests/storage/conftest.py::write_swing`**, the builder the
# Python tests trust, so a manifest or a state file here is what `save_manifest` and `save_state`
# wrote rather than a hand-typed imitation of it, and edited afterwards only where a case says so.
# Their line endings are normalised to `\n`, because `write_text` writes `\r\n` on Windows and a
# vector should not depend on the machine that recorded it; JSON reads the same either way. The
# real tree keeps the bytes on disk.
#
# **`input.versions` is the parameter Rust's `read_corpus` will take** (the plan's call 7):
# `{installed, comparable_from}`. Frozen Python's reader reads `ANALYSIS_VERSION` for itself, so
# `_run_corpus` refuses any pair but `{v, v}` at its own `v`, which is where the installed-version
# rule and P14's comparable-from rule agree.
#
# **`expected` is `{corpus, properties, narrowed}`.** `properties` are `CareerCorpus`'s derived
# counts (`distinct_swings`, `mishit_refs` and the rest), which `model_dump` leaves out and every
# report prints, so a port's are gated here rather than only through report text. Each narrowing's
# arguments are in `input.narrowings` and its answer, the same `{corpus, properties}`, in
# `expected.narrowed`: the plan's call 1 put the arguments under `expected`, where `_run_corpus`
# could not read them.
#
# **The version is `career_version`, top-level**, M34's finding for `screen_parser_version`: it is
# where `conformance.py list` and `golf-core rerecord` look. 0 is frozen Python's, which has no
# `CAREER_VERSION`. `provenance.analysis_version` names the engine whose `is_outdated` answered; a
# storage vector does not age on it.

#: Frozen Python's place in `CAREER_VERSION`'s ledger. See the section header.
_FROZEN_CAREER = 0

#: The golfer the real corpus is read for, and the only one `data/` holds.
_REAL_PLAYER = "aaron"

#: What `read_corpus` opens in a swing directory besides `analysis.json`, which travels slimmed.
_SWING_FILES = ("manifest.json", "analysis.state.json")

#: Measured at M36 P1: the real case is 480 KB as plain JSON, over the plan's ~200 KB line, and
#: 27 KB gzipped, so it is gzipped, as `spec/vectors/corpus/` is.
_REAL_SUFFIX = ".json.gz"


def build_storage_corpus(*, real: bool = True) -> list[tuple[Path, dict[str, Any]]]:
    """The corpus half of the storage family, as (path, payload) pairs. Writes nothing.

    `real=False` leaves out the case read from `data/`, which is what lets the dry-run pin in
    `tests/test_conformance.py` run on a checkout with no captures. With it, a missing `data/` is
    an error rather than a smaller family.
    """
    corpus_dir = VECTORS / "storage" / "corpus"
    out = [(corpus_dir / f"{name}.json", v) for name, v in _storage_corpus_synthetic().items()]
    if real:
        out.append((corpus_dir / f"real{_REAL_SUFFIX}", _storage_corpus_real()))
    return out


def _run_corpus(given: dict[str, Any]) -> dict[str, Any]:
    """One corpus case's input through frozen Python: the definition a port reproduces.

    `input.files` materialised into a scratch sessions root, `read_corpus` over it for
    `input.player_id`, then `narrow_to` once per entry of `input.narrowings` over that one corpus.
    """
    versions = given["versions"]
    frozen = {"installed": ANALYSIS_VERSION, "comparable_from": ANALYSIS_VERSION}
    if versions != frozen:
        raise ValueError(
            f"frozen Python's `read_corpus` answers only under {frozen}, its own installed engine; "
            f"{versions} is a question for Rust's"
        )
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch) / "sessions"
        _materialise(given["files"], root)
        return _read_and_narrow(root, given)


def _read_and_narrow(root: Path, given: dict[str, Any]) -> dict[str, Any]:
    """`_run_corpus` from the directory read on, so the real case can be read off `data/` itself."""
    from datetime import datetime

    from golf_coach.contracts.club import ClubId
    from golf_coach.storage.corpus import narrow_to, read_corpus

    corpus = read_corpus(root, given["player_id"])
    narrowed = {}
    for name, args in given["narrowings"].items():
        narrowed[name] = _corpus_answer(
            narrow_to(
                corpus,
                since=None if args["since"] is None else datetime.fromisoformat(args["since"]),
                sessions=args["sessions"],
                club=None if args["club"] is None else ClubId(args["club"]),
            )
        )
    return {**_corpus_answer(corpus), "narrowed": narrowed}


def _corpus_answer(corpus: Any) -> dict[str, Any]:
    """A `CareerCorpus` as `model_dump(mode="json")` writes it, and the counts it derives."""
    return {
        "corpus": corpus.model_dump(mode="json"),
        "properties": {
            "distinct_swings": corpus.distinct_swings,
            "distinct_shots": corpus.distinct_shots,
            "distinct_sessions": corpus.distinct_sessions,
            "untagged_swings": corpus.untagged_swings,
            "duplicates_collapsed": corpus.duplicates_collapsed,
            "shot_conflicts": corpus.shot_conflicts,
            "mishit_shots": corpus.mishit_shots,
            "mishit_refs": corpus.mishit_refs,
            "mishit_shots_unconfirmed": corpus.mishit_shots_unconfirmed,
        },
    }


def _materialise(files: dict[str, str] | None, root: Path) -> None:
    """Lay `input.files` out under `root`. `None` leaves the root absent; a `/` key is a directory.

    Written as UTF-8 bytes rather than through `write_text`, so what lands on disk is the text in
    the vector and not the text with Windows' line endings substituted into it.
    """
    if files is None:
        return
    root.mkdir(parents=True)
    for rel, text in files.items():
        parts = rel.rstrip("/").split("/")
        if rel.startswith("/") or ".." in parts or "" in parts:
            raise ValueError(f"{rel!r} is not a path inside the sessions root")
        target = root.joinpath(*parts)
        if rel.endswith("/"):
            assert text == "", f"{rel!r} names a directory and carries text"
            target.mkdir(parents=True, exist_ok=True)
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(text.encode("utf-8"))


def _read_tree(root: Path) -> dict[str, str]:
    """A scratch sessions root read back as `input.files`, `\\r\\n` normalised (the header says
    why), with an empty directory as its `/` key."""
    files: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        rel = path.relative_to(root).as_posix()
        if path.is_file():
            files[rel] = path.read_bytes().decode("utf-8").replace("\r\n", "\n")
        elif not any(path.iterdir()):
            files[f"{rel}/"] = ""
    return files


def _storage_vector(
    vector_id: str,
    given: dict[str, Any],
    *,
    note: str,
    source: str,
    run: Callable[[dict[str, Any]], dict[str, Any]] = _run_corpus,
) -> dict[str, Any]:
    """Serialize the input, run it through `run`, and pair the two (`_vector`'s rule).

    `run` is `_run_corpus` or `_run_ops`, and `provenance.recorded_by` names it, so a port reads
    there which of the two definitions a case reproduces. `provenance.analysis_version` is the
    corpus reader's alone: it names the engine whose `is_outdated` answered, and no store operation
    reads one.
    """
    import pydantic

    given_json = json.loads(json.dumps(given))
    engine = {"analysis_version": ANALYSIS_VERSION} if run is _run_corpus else {}
    return {
        "id": vector_id,
        "career_version": _FROZEN_CAREER,
        "provenance": {
            "kind": "storage",
            "oracle": "python",
            "note": note,
            "source": source,
            **engine,
            "python_version": _python_version(),
            "pydantic_version": pydantic.VERSION,
            "recorded_by": f"scripts/conformance_vectors.py::{run.__name__}",
        },
        "input": given_json,
        "expected": run(given_json),
    }


# ------------------------------------------------------------------- storage: the corpus, synthetic


def _storage_corpus_synthetic() -> dict[str, dict[str, Any]]:
    """`read_corpus`'s cases, at least one per row of the M36 plan's P1 list.

    Each note names the Python test it mirrors, or says what it pins that no test does — mostly the
    places a port reading the same files would naturally answer differently: the string tiebreaks,
    the scan order `excluded` inherits, and a manifest whose ids are trusted over its directory.
    """
    from datetime import UTC, datetime, timedelta, timezone

    from golf_coach.contracts.club import ClubId
    from golf_coach.contracts.mishit import MishitVerdict

    kit = _load_conftest(REPO / "tests" / "storage" / "conftest.py", "_conformance_storage_kit")
    metric, analysis = kit.measurement, kit.analysis_with
    lm_source, flight, population = "launch_monitor:hd_golf", "model:flight_v1", "population:golfdb"
    seven, wedge, driver = ClubId.SEVEN_IRON, ClubId.SAND_WEDGE, ClubId.DRIVER

    def at(day: int, hour: int = 12, minute: int = 0, *, tz: Any = UTC, micro: int = 0) -> Any:
        return datetime(2026, 8, day, hour, minute, 0, micro, tzinfo=tz)

    def lm(name: str, value: float, unit: str = "yards") -> dict[str, Any]:
        return metric(name, value, source=lm_source, unit=unit)

    def trusted() -> dict[str, Any]:
        return kit.flagged_shot(needs_review=False)

    pose = analysis([metric("head_sway_norm", 0.25)])
    shot_metrics = analysis(
        [metric("head_sway_norm", 0.25), lm("face_to_path_deg", 13.2, "degrees")], shot=trusted()
    )

    def with_version(value: Any) -> dict[str, Any]:
        stored = analysis([metric("head_sway_norm", 0.25)])
        stored["analysis_version"] = value
        return stored

    def club_shot(day: int, carry: float, club: Any = seven, **kw: Any) -> tuple[Any, ...]:
        """`test_corpus.py::_seven_iron`, for any club: its own clip and photo, three metrics."""
        fields: dict[str, Any] = {
            "face_on": f"clip-{club.value}-{day}",
            "shot_screen": f"photo-{club.value}-{day}",
            "club": club,
            "created_at": at(day),
            "analysis": analysis(
                [
                    lm("carry_distance_yds", carry),
                    lm("ball_speed_mph", 120.0, "mph"),
                    metric("head_sway_norm", 0.2),
                ]
            ),
        }
        fields.update(kw)
        return (f"2026-08-{day:02d}", club.value, fields)

    cases: dict[str, dict[str, Any]] = {}

    def case(
        name: str,
        note: str,
        swings: Any = (),
        *,
        edits: dict[str, str | None] | None = None,
        player: str = _REAL_PLAYER,
        narrowings: dict[str, dict[str, Any]] | None = None,
        files: Any = True,
    ) -> None:
        """Record one case. `files` is the tree `swings` and `edits` build, unless a case passes a
        tree of its own, or `None` for a root that does not exist."""
        assert name not in cases, f"two storage corpus cases are named {name}"
        tree = _sessions_tree(kit, swings, edits) if files is True else files
        given = {
            "player_id": player,
            "versions": {"installed": ANALYSIS_VERSION, "comparable_from": ANALYSIS_VERSION},
            "files": tree,
            "narrowings": {
                key: {"since": None, "sessions": None, "club": None, **args}
                for key, args in (narrowings or {}).items()
            },
        }
        cases[name] = _storage_vector(
            f"storage/corpus/{name}",
            given,
            note=note,
            source="tests/storage/conftest.py::write_swing, edited where the note says",
        )

    def s(session_id: str, swing_id: str, **fields: Any) -> tuple[str, str, dict[str, Any]]:
        return (session_id, swing_id, fields)

    # ------------------------------------------------------------- the root, and who swung it

    case(
        "missing-sessions-dir",
        "a sessions root that does not exist reads as an empty corpus, not an error "
        "(`test_a_missing_sessions_directory_is_not_an_error`); a port's directory read must not "
        "raise here",
        files=None,
    )
    case(
        "empty-sessions-dir",
        "a root that exists and holds nothing. Frozen Python answers it as it answers a missing "
        "one, and a port's directory listing meets something different",
        files={},
    )
    case(
        "dotted-and-stray-entries",
        "what `list_session_ids` and `get_session` skip: a dotted session (`.incoming`, which "
        "holds a whole swing here), a dotted swing directory, and stray files at both levels. A "
        "session holding only `session.json`, and an empty session directory, are still sessions "
        "scanned",
        [
            s("2026-08-10", "1", face_on="clip-a", analysis=pose),
            s(".incoming", "1", face_on="clip-b", analysis=pose),
            s("2026-08-10", ".partial", face_on="clip-c", analysis=pose),
        ],
        edits={
            "notes.txt": "not a session",
            "2026-08-10/README": "not a swing",
            "2026-08-10/session.json": '{"player_id": "aaron"}',
            "2026-08-12/session.json": '{"player_id": "aaron"}',
            "2026-08-13/": "",
        },
    )
    case(
        "unknown-player",
        "a golfer with no swings is an empty corpus whose scan still counted the others "
        "(`test_an_unknown_golfer_reads_as_an_empty_corpus`)",
        [s("2026-08-10", "1", face_on="clip-a", analysis=pose)],
        player="nobody",
    )
    case(
        "unattributed",
        "a swing naming nobody is itemised, because it may be this golfer's "
        "(`test_unattributed_swings_are_itemised_because_they_are_repairable`)",
        [
            s("2026-08-10", "1", face_on="clip-a", analysis=pose),
            s("2026-08-10", "2", player_id=None, face_on="clip-b", analysis=pose),
        ],
    )
    case(
        "another-golfer",
        "another golfer's swing is counted and never itemised "
        "(`test_another_golfers_swings_are_counted_but_never_included`)",
        [
            s("2026-08-10", "1", face_on="clip-a", analysis=pose),
            s("2026-08-10", "2", player_id="dave", face_on="clip-b", analysis=pose),
        ],
    )

    # ------------------------------------------------------------------- identity and dedupe

    case(
        "no-face-on",
        "no face-on clip is `NO_FACE_ON`, and never a swing "
        "(`test_a_swing_with_no_face_on_clip_can_never_carry_a_pose_measurement`); the second has "
        "no shot photo either, and is excluded for the clip alone",
        [
            s("2026-08-10", "1", face_on=None, analysis=pose),
            s("2026-08-10", "2", face_on=None, shot_screen=None, analysis=pose),
            s("2026-08-10", "3", face_on="clip-a", analysis=pose),
        ],
    )
    case(
        "duplicate-three-uploads",
        "one clip in three directories, scanned in an order that is not arrival order: the "
        "earliest arrival survives and the other two are `DUPLICATE`, listed in arrival order "
        "(`test_the_earliest_arrival_survives_and_names_what_it_absorbed`)",
        [
            s("2026-08-07", "1", face_on="clip-a", created_at=at(10), analysis=pose),
            s("2026-08-09", "2", face_on="clip-a", created_at=at(7), analysis=pose),
            s("2026-08-10", "1", face_on="clip-a", created_at=at(9), analysis=pose),
        ],
    )
    case(
        "duplicate-same-second",
        "two arrivals in the same second, which `_arrival`'s tiebreak decides: the session id, "
        "then the swing id, **as strings**. So swing `10` survives swing `9` in one session, "
        "though `get_session` scanned `9` first — a numeric tiebreak would keep the other",
        [
            s("2026-08-09", "1", face_on="clip-a", created_at=at(9), analysis=pose),
            s("2026-08-10", "1", face_on="clip-a", created_at=at(9), analysis=pose),
            s("2026-08-11", "9", face_on="clip-b", created_at=at(11), analysis=pose),
            s("2026-08-11", "10", face_on="clip-b", created_at=at(11), analysis=pose),
        ],
    )
    case(
        "duplicate-across-offsets",
        "`_arrival` sorts on `created_at.isoformat()`, a **string** in the manifest's own offset, "
        "not on the instant. Clip a: 15:00+05:30 (09:30Z) loses to 12:00Z, because '12' < '15'. "
        "Clip b: 08:00-05:00 (13:00Z) beats 12:00Z, and its later time becomes `captured_at`. "
        "Clip c: a whole second sorts before a fraction because isoformat writes `+00:00` and "
        "'+' < '.'; a key spelled with `Z` would reverse it. `swings` itself then sorts by "
        "instant",
        [
            s("2026-08-10", "1", face_on="clip-a", created_at=at(10), analysis=pose),
            s(
                "2026-08-10",
                "2",
                face_on="clip-a",
                created_at=at(10, 15, tz=timezone(timedelta(hours=5, minutes=30))),
                analysis=pose,
            ),
            s(
                "2026-08-11",
                "1",
                face_on="clip-b",
                created_at=at(11, 8, tz=timezone(timedelta(hours=-5))),
                analysis=pose,
            ),
            s("2026-08-11", "2", face_on="clip-b", created_at=at(11), analysis=pose),
            s("2026-08-12", "1", face_on="clip-c", created_at=at(12, micro=500000), analysis=pose),
            s("2026-08-12", "2", face_on="clip-c", created_at=at(12), analysis=pose),
        ],
    )
    case(
        "conflicting-shot-photos",
        "re-uploads carrying other photos: the survivor's photo is never a conflict, the rest "
        "are, sorted and named once each "
        "(`test_one_clip_with_two_shot_photos_is_a_conflict_not_a_second_reading`). Clip b's "
        "survivor has no photo, so every photo on its duplicates conflicts",
        [
            s("2026-08-07", "1", face_on="clip-a", shot_screen="shot-a", created_at=at(7),
              analysis=shot_metrics),
            s("2026-08-08", "1", face_on="clip-a", shot_screen="shot-c", created_at=at(8),
              analysis=shot_metrics),
            s("2026-08-09", "1", face_on="clip-a", shot_screen="shot-b", created_at=at(9),
              analysis=shot_metrics),
            s("2026-08-10", "1", face_on="clip-a", shot_screen="shot-a", created_at=at(10),
              analysis=shot_metrics),
            s("2026-08-11", "1", face_on="clip-a", shot_screen=None, created_at=at(11),
              analysis=shot_metrics),
            s("2026-08-07", "2", face_on="clip-b", shot_screen=None, created_at=at(7),
              analysis=shot_metrics),
            s("2026-08-08", "2", face_on="clip-b", shot_screen="shot-z", created_at=at(8),
              analysis=shot_metrics),
        ],
    )
    case(
        "one-photo-two-clips",
        "two real swings sharing one photo are two pose samples and one shot sample, the "
        "simulated flight included "
        "(`test_two_clips_sharing_a_shot_photo_is_two_pose_samples_and_one_shot_sample`, "
        "`test_a_simulated_flight_dedupes_on_the_shot_photo_and_not_the_clip`)",
        [
            s("2026-08-10", str(n), face_on=f"clip-{n}", shot_screen="shot-a",
              analysis=analysis(
                  [
                      metric("head_sway_norm", 0.25),
                      lm("face_to_path_deg", 13.2, "degrees"),
                      metric("flight_carry_yds", 141.2, source=flight, unit="yards"),
                  ],
                  shot=trusted(),
              ))
            for n in (1, 2)
        ],
    )

    # ------------------------------------------------------------- the analysis and its state

    case(
        "not-analyzed",
        "no `analysis.json` is a swing and not a sample "
        "(`test_an_unanalyzed_swing_is_counted_as_a_swing_but_not_as_a_sample`)",
        [s("2026-08-10", "1", face_on="clip-a", analysis=None)],
    )
    case(
        "corrupt-analysis",
        "an `analysis.json` `load_analysis` cannot use reads as no analysis at all: truncated "
        "JSON, an array, and `null` (`json.loads` succeeds on the last two and they are not a "
        "dict). Each still has a state file, which is never read for an unanalyzed swing",
        [s("2026-08-10", str(n), face_on=f"clip-{n}", analysis=pose) for n in (1, 2, 3)],
        edits={
            "2026-08-10/1/analysis.json": '{"analysis_version": 16, "swing": {"measure',
            "2026-08-10/2/analysis.json": "[1, 2, 3]",
            "2026-08-10/3/analysis.json": "null",
        },
    )
    case(
        "stale",
        "inputs re-uploaded since the run: a real swing that contributes nothing "
        "(`test_a_stale_analysis_is_a_real_swing_that_contributes_nothing`), beside a fresh one",
        [
            s("2026-08-10", "1", face_on="clip-a", analysis=pose, stale=True),
            s("2026-08-10", "2", face_on="clip-b", analysis=pose),
        ],
    )
    case(
        "unreadable-state",
        "a state file `load_state` cannot read is no state, so **not** stale: truncated JSON (1), "
        "a status outside the literal (2), and no file (3). A readable state with no `inputs` "
        "(4) defaults them to `{}`, which matches no manifest, so it **is** stale",
        [s("2026-08-10", str(n), face_on=f"clip-{n}", analysis=pose) for n in (1, 2, 3, 4)],
        edits={
            "2026-08-10/1/analysis.state.json": '{"status": "done", "inputs": {"face_on"',
            "2026-08-10/2/analysis.state.json": '{"status": "finished", "inputs": {}}',
            "2026-08-10/3/analysis.state.json": None,
            "2026-08-10/4/analysis.state.json": '{"status": "done"}',
        },
    )
    versions: list[Any] = [None, 0, 15, 16.0, "16", -1, True, ANALYSIS_VERSION, 17]
    outdated_swings = []
    for n, value in enumerate(versions, start=1):
        stored = analysis([metric("head_sway_norm", 0.25)], version=None)
        if value is not None:
            stored["analysis_version"] = value
        outdated_swings.append(s("2026-08-10", str(n), face_on=f"clip-{n}", analysis=stored))
    case(
        "outdated-versions",
        "`stored_analysis_version` over every shape it meets: no key, 0, 15, a float 16.0, a "
        "string '16', -1 and `true` all read as an engine older than 16 and are `OUTDATED`, "
        "stamped with what the stamp read as (0 for all but 15). 16 is current, and 17, newer "
        "than the installed engine, is not outdated under frozen Python's `<`. "
        "(`test_an_artifact_from_an_older_engine_is_a_real_swing_that_contributes_nothing`)",
        outdated_swings,
    )
    case(
        "stale-and-outdated",
        "both axes at once: one swing, excluded twice, `STALE` before `OUTDATED`",
        [
            s("2026-08-10", "1", face_on="clip-a", stale=True,
              analysis=analysis([metric("head_sway_norm", 0.25)], version=15)),
        ],
    )
    case(
        "swing-not-a-dict",
        "an analysis whose `swing` is `null`, an array, absent, or a dict with no "
        "`measurements`, and a current one whose measurements are `[]`: each is analyzed and "
        "counted, with no measurements and no review flag "
        "(`test_a_current_artifact_with_no_measurements_is_reported_not_reconstructed`)",
        [
            s("2026-08-10", "1", face_on="clip-1",
              analysis={"analysis_version": ANALYSIS_VERSION, "swing": None}),
            s("2026-08-10", "2", face_on="clip-2",
              analysis={"analysis_version": ANALYSIS_VERSION, "swing": [1]}),
            s("2026-08-10", "3", face_on="clip-3",
              analysis={"analysis_version": ANALYSIS_VERSION}),
            s("2026-08-10", "4", face_on="clip-4",
              analysis={"analysis_version": ANALYSIS_VERSION, "swing": {"shot": None}}),
            s("2026-08-10", "5", face_on="clip-5", analysis=analysis(None)),
        ],
    )
    entries: list[Any] = [
        metric("head_sway_norm", 0.25),
        {"name": "no_unit", "value": 1.0, "source": "pose:face_on"},
        {"name": "null_value", "value": None, "unit": "ratio", "source": "pose:face_on"},
        {"name": "word_value", "value": "abc", "unit": "ratio", "source": "pose:face_on"},
        {"value": 1.0, "unit": "ratio", "source": "pose:face_on"},
        "not an entry",
        42,
        {"name": "extra_key", "value": 0.5, "unit": "ratio", "source": "pose:face_on", "x": 1},
        {"name": "no_detail", "value": 0.75, "unit": "ratio", "source": "pose:face_on"},
    ]
    case(
        "invalid-measurements",
        "`_measurements` keeps the entries `Measurement` validates and drops the rest one by "
        "one: no unit, a null or non-numeric value, no name, and entries that are not objects "
        "are dropped; an unknown key is ignored and a missing `detail` defaults to ''. Swing 2's "
        "`measurements` is an object, not a list, so it has none. No entry leans on a lax "
        "pydantic coercion (the plan's call 9)",
        [
            s("2026-08-10", "1", face_on="clip-1",
              analysis={"analysis_version": ANALYSIS_VERSION, "swing": {"measurements": entries}}),
            s("2026-08-10", "2", face_on="clip-2",
              analysis={"analysis_version": ANALYSIS_VERSION,
                        "swing": {"measurements": {"head_sway_norm": 0.2}}}),
        ],
    )
    unvalidated = trusted()
    del unvalidated["timestamp"]
    unprovenanced = {**trusted(), "provenance": None}
    review_metrics = [
        metric("head_sway_norm", 0.25),
        lm("face_to_path_deg", 13.2, "degrees"),
        metric("flight_carry_yds", 141.2, source=flight, unit="yards"),
    ]
    case(
        "shot-review",
        "`_needs_review` over the attached shot: a flagged parse (1) and a shot that no longer "
        "validates (3, no timestamp) take every shot-keyed sample with them, the flight "
        "included; a trusted parse (2), a shot that is not an object (4) and one with no "
        "provenance (5) do not (`test_a_flagged_shot_contributes_to_no_launch_monitor_count`, "
        "`test_a_flagged_parse_takes_the_simulated_flight_with_it`)",
        [
            s("2026-08-10", str(n), face_on=f"clip-{n}", shot_screen=f"photo-{n}",
              analysis=analysis(review_metrics, shot=shot))
            for n, shot in enumerate(
                [kit.flagged_shot(), trusted(), unvalidated, "screen", unprovenanced], start=1
            )
        ],
    )

    # --------------------------------------------------------------------------- the mishits

    case(
        "mishit-four-carries",
        "four distinct carries is under `MISHIT_MIN_CLEAN_SHOTS`, so a 20-yard top stays in "
        "(`test_the_rule_stays_quiet_below_the_clean_sample_floor`)",
        [club_shot(day, carry) for day, carry in [(7, 155.0), (8, 160.0), (9, 158.0), (10, 20.0)]],
    )
    case(
        "mishit-five-carries",
        "five distinct carries cross the floor: median 158 of an odd count, floor 79, and the "
        "20-yard top leaves the carry count while its ball speed and pose stay "
        "(`test_a_topped_shot_leaves_the_carry_average_and_stays_in_every_other`)",
        [
            club_shot(day, carry)
            for day, carry in [(7, 155.0), (8, 160.0), (9, 158.0), (10, 162.0), (11, 20.0)]
        ],
    )
    case(
        "mishit-two-clubs",
        "each club against its own median. 7 iron: six carries, median (158 + 159) / 2 = 158.5, "
        "floor 79.25, so its 90 stays. Driver: seven, median 248, floor exactly 124.0 — its 124 "
        "stays (the rule is a strict `<`) and its 110 goes. 90 would have gone under the "
        "driver's floor. Narrowed to each club, and to one never hit "
        "(`test_a_club_narrowing_carries_the_mishit_flags`)",
        [
            club_shot(day, carry)
            for day, carry in [
                (7, 155.0), (8, 160.0), (9, 158.0), (10, 162.0), (11, 159.0), (12, 90.0),
            ]
        ]
        + [
            club_shot(day, carry, driver)
            for day, carry in [
                (13, 250.0), (14, 246.0), (15, 255.0), (16, 248.0), (17, 124.0), (18, 110.0),
                (19, 260.0),
            ]
        ],
        narrowings={
            "club-7i": {"club": seven.value},
            "club-driver": {"club": driver.value},
            "club-sw-unhit": {"club": wedge.value},
        },
    )
    case(
        "mishit-verdicts",
        "the golfer's verdict wins both ways. Carries 155 160 158 162 159, a CLEARED 20, a "
        "CONFIRMED 120 and an unruled 25: all eight are in the median ((155 + 158) / 2 = 156.5, "
        "floor 78.25) because the verdict decides `is_mishit`, not the median. The 20 counts, "
        "the 120 and the 25 do not, and only the 25 is unconfirmed "
        "(`test_a_cleared_verdict_puts_an_auto_flagged_shot_back`, "
        "`test_a_confirmed_verdict_removes_a_shot_the_rule_would_have_kept`)",
        [
            club_shot(day, carry)
            for day, carry in [(7, 155.0), (8, 160.0), (9, 158.0), (10, 162.0), (11, 159.0)]
        ]
        + [
            club_shot(12, 20.0, mishit=MishitVerdict.CLEARED),
            club_shot(13, 120.0, mishit=MishitVerdict.CONFIRMED),
            club_shot(14, 25.0),
        ],
    )
    reused_iron = [
        club_shot(day, carry)
        for day, carry in [(7, 155.0), (8, 160.0), (9, 158.0), (10, 20.0)]
    ]
    reused_iron.append(club_shot(11, 20.0, shot_screen=f"photo-{seven.value}-10"))
    reused_driver = [
        club_shot(day, carry, driver)
        for day, carry in [(12, 250.0), (13, 245.0), (14, 255.0), (15, 248.0), (16, 90.0)]
    ]
    reused_driver.append(club_shot(17, 90.0, driver, shot_screen=f"photo-{driver.value}-16"))
    case(
        "mishit-reused-photo",
        "the median counts one carry per distinct photo, first seen. 7 iron: five swings but "
        "four photos, so the rule stays quiet. Driver: six swings, five photos, median 248: the "
        "photo both 90s share is one mishit shot, and both swings are named in `mishit_refs`",
        reused_iron + reused_driver,
    )
    case(
        "mishit-ineligible",
        "five clean 7 irons (median 159, floor 79.5) beside five 25-yard carries the rule must "
        "not see: a flagged parse, an outdated engine, a stale analysis, no shot photo, and no "
        "club. None is flagged and none moves the median; admitted, all five would drag it to "
        "90 (floor 45) and be flagged with it "
        "(`test_a_flagged_parse_is_neither_a_mishit_nor_in_the_median`)",
        [
            club_shot(day, carry)
            for day, carry in [(7, 155.0), (8, 160.0), (9, 158.0), (10, 162.0), (11, 159.0)]
        ]
        + [
            club_shot(12, 25.0, analysis=analysis(
                [lm("carry_distance_yds", 25.0)], shot=kit.flagged_shot())),
            club_shot(13, 25.0, analysis=analysis([lm("carry_distance_yds", 25.0)], version=15)),
            club_shot(14, 25.0, stale=True),
            club_shot(15, 25.0, shot_screen=None),
            s("2026-08-16", "untagged", face_on="clip-untagged", shot_screen="photo-untagged",
              created_at=at(16), analysis=analysis([lm("carry_distance_yds", 25.0)])),
        ],
    )

    # --------------------------------------------------------------------- sources and order

    every_source = analysis(
        [
            metric("head_sway_norm", 0.25),
            metric("pelvis_turn_dtl", 30.0, source="pose:down_the_line", unit="degrees"),
            metric("flight_carry_yds", 141.2, source=flight, unit="yards"),
            metric("club_lag_deg", 4.0, source="radar:trackman", unit="degrees"),
            metric("tour_joint_distance", 2.4, source=population, unit="sd"),
            lm("face_to_path_deg", 13.2, "degrees"),
        ],
        shot=trusted(),
    )
    case(
        "every-source",
        "two untagged clips sharing one photo, each carrying every provenance: `pose:` keys on "
        "the clip (2), `pose:down_the_line` on nothing and is not unknown, `model:` and "
        "`launch_monitor:` on the photo (1), and an unknown source and `population:golfdb` on the "
        "swing (2), both named in `unknown_sources` "
        "(`test_a_placement_is_still_an_unknown_source_and_that_is_the_deferral`)",
        [
            s("2026-08-10", str(n), face_on=f"clip-{n}", shot_screen="shot-a",
              analysis=every_source)
            for n in (1, 2)
        ],
    )
    case(
        "sort-order",
        "`swings` sorts by instant, then session id, then swing id as a string: swings `10`, "
        "`2` and `x` at one instant in one session sort in that order, and 20:00-05:00 on the "
        "8th (01:00Z on the 9th) sorts after 00:30Z on the 9th, which a string sort would "
        "reverse. `excluded` keeps the scan order instead: `get_session`'s numeric sort, "
        "non-numeric names last, so session 2026-08-12's unattributed swings are 2, 9, 10, x "
        "(`test_swings_are_ordered_oldest_first`)",
        [
            s("2026-08-11", "1", face_on="clip-11", created_at=at(11), analysis=pose),
            s("2026-08-07", "1", face_on="clip-7", created_at=at(7), analysis=pose),
            s("2026-08-09", "2", face_on="clip-9-2", created_at=at(9), analysis=pose),
            s("2026-08-09", "10", face_on="clip-9-10", created_at=at(9), analysis=pose),
            s("2026-08-09", "x", face_on="clip-9-x", created_at=at(9), analysis=pose),
            s("2026-08-09", "3", face_on="clip-9-3", created_at=at(9, 0, 30), analysis=pose),
            s("2026-08-08", "1", face_on="clip-8", analysis=pose,
              created_at=at(8, 20, tz=timezone(timedelta(hours=-5)))),
        ]
        + [
            s("2026-08-12", swing_id, player_id=None, face_on=f"clip-12-{swing_id}",
              analysis=pose)
            for swing_id in ("10", "9", "x", "2")
        ],
    )
    case(
        "narrowing",
        "`narrow_to` by a window (inclusive, compared by instant, so the same instant written "
        "in +05:30 narrows identically), by sessions (one that does not exist, and the empty "
        "collection, which keeps nothing where `None` keeps everything), by club (one never hit "
        "is empty), and a window with a club. Counts, `outdated_swings` and "
        "`analyzed_without_measurements` are recomputed; `excluded` and the scan counters are not "
        "(`test_narrowing_recomputes_the_counts_it_leaves_behind`, `test_club_and_since_compose`, "
        "`test_narrowing_leaves_the_scan_counters_describing_the_whole_read`)",
        [
            club_shot(7, 155.0),
            club_shot(8, 70.0, wedge),
            club_shot(9, 158.0),
            s("2026-08-09", "untagged", face_on="clip-untagged", created_at=at(9, 13),
              analysis=pose),
            club_shot(10, 160.0, analysis=analysis([metric("head_sway_norm", 0.3)], version=15)),
            s("2026-08-10", "empty", face_on="clip-empty", club=seven, created_at=at(10, 13),
              analysis=analysis(None)),
            s("2026-08-10", "nobody", player_id=None, face_on="clip-nobody", analysis=pose),
        ],
        narrowings={
            "since-the-9th": {"since": "2026-08-09T12:00:00Z"},
            "since-the-9th-in-ist": {"since": "2026-08-09T17:30:00+05:30"},
            "sessions": {"sessions": ["2026-08-07", "2026-08-10", "2026-09-01"]},
            "no-sessions": {"sessions": []},
            "club-7i": {"club": seven.value},
            "club-driver-unhit": {"club": driver.value},
            "since-the-8th-7i": {"since": "2026-08-08T00:00:00Z", "club": seven.value},
            "no-filter": {},
        },
    )

    # ---------------------------------------------------------------------- the manifests

    misplaced = _sessions_tree(
        kit,
        [
            s("2026-08-10", "1", face_on="clip-a", analysis=pose),
            s("2026-08-10", "7", face_on="clip-b", analysis=pose),
        ],
        None,
    )
    for name in (*_SWING_FILES, "analysis.json"):
        misplaced[f"2026-08-10/2/{name}"] = misplaced.pop(f"2026-08-10/7/{name}")
    case(
        "manifest-names-another-dir",
        "directory 2 holds a manifest naming swing 7, beside an analysis. `read_corpus` trusts "
        "the manifest's ids and looks for the analysis in `2026-08-10/7/`, which does not exist, "
        "so the swing is `NOT_ANALYZED` under ref 2026-08-10/7. A port that read from the "
        "directory it listed would count it",
        files=dict(sorted(misplaced.items())),
    )
    manifests = _sessions_tree(
        kit,
        [s("2026-08-10", str(n), face_on=f"clip-{n}", analysis=pose) for n in range(1, 10)],
        None,
    )

    def edit_manifest(n: int, change: Any) -> None:
        path = f"2026-08-10/{n}/manifest.json"
        document = json.loads(manifests[path])
        change(document)
        manifests[path] = json.dumps(document, indent=2)

    manifests["2026-08-10/2/manifest.json"] = manifests["2026-08-10/2/manifest.json"][:80]
    for name in (*_SWING_FILES, "analysis.json"):
        del manifests[f"2026-08-10/3/{name}"]
    manifests["2026-08-10/3/"] = ""
    edit_manifest(4, lambda m: m.pop("created_at"))
    edit_manifest(5, lambda m: m.update(club="9w"))
    edit_manifest(6, lambda m: m["roles"].update(side_on=m["roles"]["face_on"]))
    edit_manifest(7, lambda m: m.update(phone_model="iPhone 15"))
    edit_manifest(8, lambda m: m.update(mishit="maybe"))
    manifests["2026-08-10/9/manifest.json"] = "[]"
    case(
        "corrupt-manifests",
        "`load_manifest` is tolerant, so each of these is skipped and not seen: truncated JSON "
        "(2), no manifest at all (3), no `created_at` (4), a club no `ClubId` names (5), a role "
        "no `Role` names (6), a verdict no `MishitVerdict` names (8), and an array (9). An "
        "unknown key (7) is ignored and the manifest reads. `swing_dirs_seen` is 2",
        files=dict(sorted(manifests.items())),
    )
    return cases


def _sessions_tree(
    kit: Any,
    swings: Any,
    edits: dict[str, str | None] | None,
) -> dict[str, str]:
    """`swings` written by `write_swing` into a scratch root and read back as `input.files`, then
    `edits` applied: a text replaces or adds a file, and `None` removes one."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch) / "sessions"
        root.mkdir()
        for session_id, swing_id, fields in swings:
            kit.write_swing(root, session_id, swing_id, **fields)
        files = _read_tree(root)
    for path, text in (edits or {}).items():
        if text is None:
            del files[path]  # a removal that removes nothing is a typo in the case
        else:
            files[path] = text
    return dict(sorted(files.items()))


# ------------------------------------------------------------------------ storage: the real corpus


def _storage_corpus_real() -> dict[str, Any]:
    """The sessions on disk, as one case, verified to read exactly as `data/` itself reads.

    Each `analysis.json` travels slimmed to the keys `read_corpus` reads (the plan's finding 11),
    which is what keeps the case small. That is a claim about the reader, and the verify is what
    holds it: `read_corpus` and every narrowing over the slim tree must equal the same calls over
    `data/processed/sessions/`, or this raises and nothing is built. Narrowed once per club the
    corpus holds, in `ClubId`'s order.
    """
    from golf_coach.contracts.club import ClubId
    from golf_coach.storage.corpus import read_corpus

    if not SESSIONS.is_dir():
        raise AssertionError(f"{_rel(SESSIONS)} is missing — the real corpus is read from it")
    clubs = {swing.club for swing in read_corpus(SESSIONS, _REAL_PLAYER).swings}
    given = {
        "player_id": _REAL_PLAYER,
        "versions": {"installed": ANALYSIS_VERSION, "comparable_from": ANALYSIS_VERSION},
        "files": _real_sessions_tree(SESSIONS),
        "narrowings": {
            f"club-{club.value}": {"since": None, "sessions": None, "club": club.value}
            for club in ClubId
            if club in clubs
        },
    }
    vector = _storage_vector(
        "storage/corpus/real",
        given,
        note=(
            "every session on disk: manifests, state files and session files as written, each "
            "analysis.json slimmed to analysis_version, swing.measurements and swing.shot, "
            "verified to read exactly as data/processed/sessions/ does"
        ),
        source=_rel(SESSIONS),
    )
    direct = _read_and_narrow(SESSIONS, vector["input"])
    if direct != vector["expected"]:
        from conformance import compare_results

        diffs = compare_results(direct, vector["expected"], "real") or ["(float bits differ)"]
        raise AssertionError(
            "the slimmed tree does not read as data/ does — "
            + "; ".join(str(d) for d in diffs[:5])
        )
    return vector


def _real_sessions_tree(root: Path) -> dict[str, str]:
    """`input.files` for the real sessions root: what `read_corpus` opens, nothing it skips.

    Dotted directories are left out because `list_session_ids` and `get_session` never enter
    them, and a directory left with nothing to carry keeps its `/` key so it is still listed.
    """
    files: dict[str, str] = {}
    for session in sorted(p for p in root.iterdir() if p.is_dir() and not p.name.startswith(".")):
        carried = len(files)
        if (session / "session.json").is_file():
            files[f"{session.name}/session.json"] = _raw_text(session / "session.json")
        for swing in sorted(
            p for p in session.iterdir() if p.is_dir() and not p.name.startswith(".")
        ):
            prefix = f"{session.name}/{swing.name}/"
            before = len(files)
            for name in _SWING_FILES:
                if (swing / name).is_file():
                    files[prefix + name] = _raw_text(swing / name)
            if (swing / "analysis.json").is_file():
                files[prefix + "analysis.json"] = _slim_analysis(_raw_text(swing / "analysis.json"))
            if len(files) == before:
                files[prefix] = ""
        if len(files) == carried:
            files[f"{session.name}/"] = ""
    return files


def _raw_text(path: Path) -> str:
    return path.read_bytes().decode("utf-8")


def _slim_analysis(text: str) -> str:
    """An `analysis.json` cut to `analysis_version`, `swing.measurements` and `swing.shot`.

    Each key only where the file has it, since an absent `analysis_version` reads as 0 and an
    absent `swing` as no measurements, and adding either would change the answer. A file that is
    not a JSON object travels as it is, because it is read as no analysis either way.
    """
    try:
        loaded = json.loads(text)
    except ValueError:
        return text
    if not isinstance(loaded, dict):
        return text
    slim: dict[str, Any] = {}
    if "analysis_version" in loaded:
        slim["analysis_version"] = loaded["analysis_version"]
    if "swing" in loaded:
        swing = loaded["swing"]
        slim["swing"] = (
            {key: swing[key] for key in ("measurements", "shot") if key in swing}
            if isinstance(swing, dict)
            else swing
        )
    return json.dumps(slim)


# ================================================================== storage: the store operations
#
# **The other half of the storage family: what the stores do to a directory** (M36 P2, the plan's
# calls 1 and 4). A case is a tree and a sequence of store calls on it. `input.files` is the store
# root as raw text, laid out by `_materialise` exactly as a corpus case's sessions root is, and
# `input.ops` is the calls, each `{op, args, now}`. `expected.results` holds what each call
# returned, as `{"returned": value}`, or what it raised, as `{"raised": {type, message}}`, and
# `expected.files` is the whole root after the last call, read back by `_read_tree` (`null` if it
# still does not exist). The sequence carries on past a raise, because what a refused write left
# on disk is the point of those cases.
#
# **Every store in a case is opened on the one root**, and an op names its store:
# `bundle.assign_from_path`, `bag.set_entry`, `golfer.get_or_create`, `shot.put`. One root is how
# `data/processed/golfers/` already holds two stores, and it is what lets a case put a bag beside
# a golfer. `slugify` is the one op with no store, and its cases have no root at all. A bundle
# case's scratch uploads sit where `api/app.py` streams them, `.incoming/<name>.part` under the
# root, and are in `input.files` from the start: an upload's `tmp_path` names one.
#
# **The clock is an argument** (call 4). Rust's store operations take `now` from their caller;
# frozen Python's read `datetime.now(tz=UTC)` for themselves. So `_frozen_clock` replaces the
# `datetime` name in the three store modules that stamp, for one op, with a subclass whose `now()`
# answers that op's `now` in the zone asked for. `src/` is not edited. An op that cannot stamp is
# recorded with `now: null`, and the clock then raises if it is read, so a `null` in a vector is a
# claim the recorder checked rather than an omission. One op reads one instant however many times
# it asks, which is why `created_at`, `received_at` and `updated_at` agree on a new swing here and
# differ by microseconds on disk.
#
# **A JSON file in `expected.files` is compared as a value, anything else as text** (call 1). What
# pins a store's write is the value pydantic serialized, the timestamps' spelling included since
# they are strings, and not its key order, indentation or line endings. Each op's `now` moves on
# by a minute and a cycle of microseconds that pydantic spells differently (none, trailing zeros,
# one, six nines), so the written files carry every spelling `contracts::time::Timestamp` (P4) has
# to write.
#
# **A raise is recorded by class and message, and the message only where this repo wrote it.** The
# bag store's own `ValueError` names the bag's path, so the scratch root is spelled `<root>/` in
# it. A pydantic `ValidationError` whose every error is a validator's own `ValueError` records
# those messages, joined by `; `. One carrying any of pydantic's own errors (bad JSON, a missing
# key) records `message: null`, because that text is pydantic's and no port is asked to match it.
#
# **What no case reaches, on purpose**: a `str.isdigit` swing directory `int()` cannot parse (`²`
# crashes `get_session`), two directories `_swing_sort_key` ties (`7` and `007`, which leaves the
# order to the filesystem), an original filename with a path separator in it (`Path.suffix` reads
# `\` differently on Windows and POSIX, and the recorder ran on Windows), a key that differs from
# a file on disk only in case (Windows' filesystem finds `aaron.bag.json` for `Aaron`, so the bag
# store's write guard fires there where `Bag`'s slug validator fires on Linux), and a `player_id`
# that climbs out of the root other than `test_bag_store.py`'s own `../aaron`, which the validator
# refuses before anything is written. Each would record this machine rather than the store.

#: Where an op case's clock starts. `_OpsCase.tick` moves it on by a minute and one of these.
_OPS_EPOCH = "2026-08-06T12:00:00Z"
_OPS_TICK_MICROS = (0, 120000, 1, 999999, 500000, 345678)

#: The bundle cases' session, `tests/storage/test_bundle_store.py::_SESSION`.
_OPS_SESSION = "2026-08-06"


def build_storage(*, real: bool = True) -> list[tuple[Path, dict[str, Any]]]:
    """The whole storage family, built in full before anything is returned. Writes nothing.

    `regenerate --storage-once` writes what this returns, and only once it has returned: a failed
    build, the real corpus's verify included, leaves `spec/` untouched (P1's finding 6).
    """
    return build_storage_corpus(real=real) + build_storage_ops()


def build_storage_ops() -> list[tuple[Path, dict[str, Any]]]:
    """The operation half of the storage family, as (path, payload) pairs. Writes nothing."""
    storage = VECTORS / "storage"
    out = [(storage / "bundle" / f"{name}.json", v) for name, v in _storage_bundle_cases().items()]
    out += [(storage / "stores" / f"{name}.json", v) for name, v in _storage_store_cases().items()]
    return out


def _run_ops(given: dict[str, Any]) -> dict[str, Any]:
    """One operation case's input through frozen Python: the definition a port reproduces.

    `input.files` materialised as the root, every store opened on it once, each op run in order
    under its own clock, and the root read back after the last.
    """
    table = _op_table()
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch) / "root"
        _materialise(given["files"], root)
        stores = _open_stores(root)
        results = [_run_op(table, stores, root, op) for op in given["ops"]]
        files = _read_tree(root) if root.exists() else None
    return {"results": results, "files": files}


def _open_stores(root: Path) -> dict[str, Any]:
    from golf_coach.launch_monitor.screen.store import ShotStore
    from golf_coach.storage.bag_store import BagStore
    from golf_coach.storage.bundle_store import SwingBundleStore
    from golf_coach.storage.golfer_store import GolferStore

    return {
        "bundle": SwingBundleStore(root),
        "bag": BagStore(root),
        "golfer": GolferStore(root),
        "shot": ShotStore(root),
    }


def _run_op(
    table: dict[str, Callable[..., Any]], stores: dict[str, Any], root: Path, op: dict[str, Any]
) -> dict[str, Any]:
    """One call, under its own clock. A `ValueError` is a recorded answer; anything else is a
    recorder bug and stops the build."""
    with _frozen_clock(op["now"]):
        try:
            returned = table[op["op"]](stores, root, op["args"])
        except ValueError as exc:
            return {"raised": _raised(exc, root)}
    return {"returned": returned}


def _op_table() -> dict[str, Callable[..., Any]]:
    """Every op a case may name, each `(stores, root, args) -> JSON`.

    Enum arguments travel as their values and are rebuilt here, and a model argument travels as
    its `model_dump(mode="json")` and is validated here, so a vector holds JSON a port can read and
    frozen Python is handed exactly the objects its callers hand it.
    """
    from datetime import datetime

    from golf_coach.contracts.bag import Bag, BagEntry
    from golf_coach.contracts.club import ClubId
    from golf_coach.contracts.golfer import Handedness, slugify
    from golf_coach.contracts.mishit import MishitVerdict
    from golf_coach.contracts.shot import ShotData
    from golf_coach.storage.manifest import Role

    def club(value: str | None) -> ClubId | None:
        return None if value is None else ClubId(value)

    def assign(stores: dict[str, Any], root: Path, a: dict[str, Any]) -> Any:
        result = stores["bundle"].assign_from_path(
            session_id=a["session_id"],
            role=Role(a["role"]),
            tmp_path=root.joinpath(*a["tmp_path"].split("/")),
            digest=a["digest"],
            original_filename=a["original_filename"],
            content_type=a["content_type"],
            size_bytes=a["size_bytes"],
            swing_id=a["swing_id"],
            player_id=a["player_id"],
            club=club(a["club"]),
        )
        return {
            "session_id": result.session_id,
            "swing_id": result.swing_id,
            "role": result.role.value,
            "status": result.status,
            "missing_roles": [role.value for role in result.missing_roles],
            "deduped": result.deduped,
            "player_id": result.player_id,
            "club": None if result.club is None else result.club.value,
        }

    def stamp_date(stores: dict[str, Any], root: Path, a: dict[str, Any]) -> Any:
        now = None if a["now"] is None else datetime.fromisoformat(a["now"])
        return stores["bundle"].current_session_id(now=now)

    def verdict(value: str | None) -> MishitVerdict | None:
        return None if value is None else MishitVerdict(value)

    def put(stores: dict[str, Any], root: Path, a: dict[str, Any]) -> Any:
        return stores["shot"].put(ShotData.model_validate(a["shot"])).relative_to(root).as_posix()

    return {
        "bundle.list_session_ids": lambda s, r, a: s["bundle"].list_session_ids(),
        "bundle.get_session": lambda s, r, a: _dumped(s["bundle"].get_session(a["session_id"])),
        "bundle.get_swing": lambda s, r, a: _dumped(
            s["bundle"].get_swing(a["session_id"], a["swing_id"])
        ),
        "bundle.current_session_id": stamp_date,
        "bundle.assign_from_path": assign,
        "bundle.attribute_unlabeled": lambda s, r, a: s["bundle"].attribute_unlabeled(
            a["session_id"], a["player_id"]
        ),
        "bundle.set_player": lambda s, r, a: _dumped(
            s["bundle"].set_player(a["session_id"], a["swing_id"], a["player_id"])
        ),
        "bundle.set_club": lambda s, r, a: _dumped(
            s["bundle"].set_club(a["session_id"], a["swing_id"], ClubId(a["club"]))
        ),
        "bundle.set_mishit": lambda s, r, a: _dumped(
            s["bundle"].set_mishit(a["session_id"], a["swing_id"], verdict(a["verdict"]))
        ),
        "bundle.delete_swing": lambda s, r, a: s["bundle"].delete_swing(
            a["session_id"], a["swing_id"]
        ),
        "bag.get": lambda s, r, a: _dumped(s["bag"].get(a["player_id"])),
        "bag.save": lambda s, r, a: s["bag"].save(Bag.model_validate(a["bag"])),
        "bag.set_entry": lambda s, r, a: _dumped(
            s["bag"].set_entry(a["player_id"], BagEntry.model_validate(a["entry"]))
        ),
        "bag.remove_entry": lambda s, r, a: _dumped(
            s["bag"].remove_entry(a["player_id"], ClubId(a["club"]))
        ),
        "bag.restore_entry": lambda s, r, a: _dumped(
            s["bag"].restore_entry(a["player_id"], ClubId(a["club"]))
        ),
        "golfer.get": lambda s, r, a: _dumped(s["golfer"].get(a["player_id"])),
        "golfer.list_all": lambda s, r, a: _dumped(s["golfer"].list_all()),
        "golfer.get_or_create": lambda s, r, a: _dumped(
            s["golfer"].get_or_create(a["name"], Handedness(a["handedness"]))
        ),
        "shot.put": put,
        "shot.get": lambda s, r, a: _dumped(s["shot"].get(a["key"])),
        "shot.has": lambda s, r, a: s["shot"].has(a["key"]),
        "shot.all": lambda s, r, a: _dumped(s["shot"].all()),
        "slugify": lambda s, r, a: slugify(a["name"]),
    }


def _dumped(value: Any) -> Any:
    """A store's answer as JSON: a model as `model_dump(mode="json")`, a list item by item."""
    if isinstance(value, list):
        return [_dumped(item) for item in value]
    if hasattr(value, "model_dump"):
        return value.model_dump(mode="json")
    return value


def _raised(exc: ValueError, root: Path) -> dict[str, Any]:
    """What a refused call raised, as the section header says: class, and this repo's message."""
    from pydantic import ValidationError

    if isinstance(exc, ValidationError):
        errors = exc.errors(include_url=False)
        if errors and all(error["type"] == "value_error" for error in errors):
            message: str | None = "; ".join(str(error["ctx"]["error"]) for error in errors)
        else:
            message = None
        return {"type": "ValidationError", "message": message}
    message = str(exc).replace(f"{root}{os.sep}", "<root>/")
    assert str(root.parent) not in message, f"a scratch path leaked into {message!r}"
    return {"type": type(exc).__name__, "message": message}


@contextlib.contextmanager
def _frozen_clock(now: str | None) -> Iterator[None]:
    """`datetime.now` answering `now` in the three store modules that stamp, for one op.

    A subclass rather than a stand-in object, so anything else those modules ask of the name still
    answers as `datetime` does. `now: null` makes reading the clock an error (the section header).
    """
    from datetime import datetime, tzinfo

    from golf_coach.storage import bag_store, bundle_store, golfer_store

    instant = None if now is None else datetime.fromisoformat(now)

    class Frozen(datetime):
        @classmethod
        def now(cls, tz: tzinfo | None = None) -> datetime:
            if instant is None:
                raise AssertionError("an op recorded with `now: null` read the clock")
            if tz is None:
                raise AssertionError("a store read a naive clock, which no store does today")
            return instant.astimezone(tz)

    modules: tuple[Any, ...] = (bag_store, bundle_store, golfer_store)
    saved = [module.datetime for module in modules]
    for module in modules:
        module.datetime = Frozen
    try:
        yield
    finally:
        for module, original in zip(modules, saved, strict=True):
            module.datetime = original


class _OpsCase:
    """One operation case under construction: a starting tree, then calls on it, in order.

    `tick` hands each op that may stamp the next instant of the case's clock; an op that cannot
    stamp gets `None`. An upload writes its scratch file into the starting tree, under the name
    `api/app.py` would stream it to, and is then an ordinary `bundle.assign_from_path`.
    """

    def __init__(self, files: dict[str, str] | None) -> None:
        self.files = None if files is None else dict(files)
        self.ops: list[dict[str, Any]] = []
        self._ticks = 0
        self._uploads = 0

    def tick(self) -> str:
        from datetime import datetime, timedelta

        step = timedelta(
            minutes=self._ticks,
            microseconds=_OPS_TICK_MICROS[self._ticks % len(_OPS_TICK_MICROS)],
        )
        self._ticks += 1
        return _zulu(datetime.fromisoformat(_OPS_EPOCH) + step)

    def op(self, op_name: str, /, *, stamps: bool = False, **args: Any) -> None:
        """One call. Positional-only, so `name` (`get_or_create`'s, `slugify`'s) is an arg."""
        self.ops.append({"op": op_name, "args": args, "now": self.tick() if stamps else None})

    def upload(
        self,
        role: str,
        content: str,
        *,
        session_id: str = _OPS_SESSION,
        filename: str = "clip.mov",
        content_type: str = "video/quicktime",
        swing_id: str | None = None,
        player_id: str | None = None,
        club: str | None = None,
    ) -> None:
        assert self.files is not None, "an upload needs a root to stream into"
        self._uploads += 1
        tmp_path = f".incoming/{self._uploads}.part"
        self.files[tmp_path] = content
        data = content.encode("utf-8")
        self.op(
            "bundle.assign_from_path",
            stamps=True,
            session_id=session_id,
            role=role,
            tmp_path=tmp_path,
            digest=hashlib.sha256(data).hexdigest(),
            original_filename=filename,
            content_type=content_type,
            size_bytes=len(data),
            swing_id=swing_id,
            player_id=player_id,
            club=club,
        )

    def given(self) -> dict[str, Any]:
        return {"files": self.files, "ops": self.ops}


def _zulu(when: Any) -> str:
    """An aware UTC datetime as RFC 3339 with `Z`, the spelling every op's `now` travels in."""
    return when.isoformat().replace("+00:00", "Z")


def _manifest_files(
    session_id: str,
    swing_id: str,
    roles: tuple[str, ...] = (),
    *,
    directory: str | None = None,
    day: int = 5,
    **fields: Any,
) -> dict[str, str]:
    """A swing directory as `save_manifest` writes one, with each role's file beside it.

    `directory` is where it sits under the root when that is not `<session_id>/<swing_id>`, for
    the case that pins a manifest trusted over the directory it was found in. Each role's digest is
    derived from where the role belongs, so no two pre-written files share one by accident.
    """
    from datetime import UTC, datetime

    from golf_coach.storage.manifest import Role, RoleFile, SwingManifest, content_filename

    at = datetime(2026, 8, day, 9, 30, tzinfo=UTC)
    role_files = {}
    files = {}
    for name in roles:
        role = Role(name)
        digest = hashlib.sha256(f"{session_id}/{swing_id}/{name}".encode()).hexdigest()
        filename = content_filename(role, digest, f"{name}.mov")
        role_files[role] = RoleFile(
            role=role,
            filename=filename,
            content_sha256=digest,
            original_filename=f"{name}.mov",
            content_type="video/quicktime",
            size_bytes=4,
            received_at=at,
        )
        files[filename] = "clip"
    manifest = SwingManifest(
        swing_id=swing_id,
        session_id=session_id,
        created_at=at,
        updated_at=at,
        roles=role_files,
        **fields,
    )
    files["manifest.json"] = manifest.model_dump_json(indent=2)
    prefix = f"{directory or f'{session_id}/{swing_id}'}/"
    return {prefix + name: text for name, text in files.items()}


#: The manifest `tests/storage/test_manifest.py` pins as written before `player_id`, `club` and
#: `mishit` existed: the bytes such a file on disk actually holds, not a model that carries the
#: three fields today.
_PRE_M9_MANIFEST = (
    '{"swing_id": "1", "session_id": "2026-08-07-aaron1",'
    ' "created_at": "2026-08-07T12:00:00Z", "updated_at": "2026-08-07T12:00:00Z",'
    ' "roles": {}}'
)


# --------------------------------------------------------------------- storage: the bundle store


def _storage_bundle_cases() -> dict[str, dict[str, Any]]:
    """`SwingBundleStore`'s cases: each test in `tests/storage/test_bundle_store.py`, as calls,
    and the places a port reading the same files would naturally answer differently."""
    face, dtl, screen = "face_on", "down_the_line", "shot_screen"
    seven, wedge = "7i", "pw"
    session = _OPS_SESSION
    cases: dict[str, dict[str, Any]] = {}

    def record(name: str, note: str, case: _OpsCase) -> None:
        assert name not in cases, f"two bundle cases are named {name}"
        cases[name] = _storage_vector(
            f"storage/bundle/{name}",
            case.given(),
            note=note,
            source="tests/storage/test_bundle_store.py, as store calls",
            run=_run_ops,
        )

    def where(swing_id: str, session_id: str = session) -> dict[str, str]:
        return {"session_id": session_id, "swing_id": swing_id}

    # ------------------------------------------------------------------------------ the reads

    tree = {
        **_manifest_files(session, "1", (face, dtl, screen), player_id="aaron", club=seven),
        **_manifest_files(session, "2", (face,), player_id="aaron"),
        **_manifest_files(session, "9", (face, dtl)),
        **_manifest_files(session, "10", (screen,), club=wedge, mishit="confirmed"),
        **_manifest_files(session, "x", (face,)),
        **_manifest_files(session, ".partial", (face,)),
        f"{session}/3/manifest.json": "{not json",
        f"{session}/4/face_on.abc.mov": "clip",
        f"{session}/README": "not a swing",
        "2026-08-07-aaron1/1/manifest.json": _PRE_M9_MANIFEST,
        "2026-08-10/": "",
        ".incoming/stale.part": "half an upload",
        "notes.txt": "not a session",
    }
    reads = _OpsCase(tree)
    reads.op("bundle.list_session_ids")
    reads.op("bundle.get_session", session_id=session)
    reads.op("bundle.get_session", session_id="2026-08-09")
    reads.op("bundle.get_session", session_id="2026-08-10")
    reads.op("bundle.get_swing", **where("10"))
    reads.op("bundle.get_swing", **where("3"))
    reads.op("bundle.get_swing", **where("4"))
    reads.op("bundle.get_swing", **where("99"))
    reads.op("bundle.get_swing", **where("1", "2026-08-07-aaron1"))
    reads.op("bundle.current_session_id", now="2026-08-07T00:30:00+05:30")
    reads.op("bundle.current_session_id", now="2026-08-06T23:30:00-05:00")
    reads.op("bundle.current_session_id", stamps=True, now=None)
    record(
        "reads",
        "`list_session_ids` is lexical and skips dotted directories and files; `get_session` "
        "sorts numeric swing directories numerically (1, 2, 9, 10) and the rest after, and skips "
        "a dotted directory, a stray file, a corrupt manifest (3) and a directory with none (4); "
        "a missing and an empty session are both []; a manifest written before `player_id`, "
        "`club` and `mishit` existed loads with all three None; `current_session_id` formats "
        "`now` in its own offset, so 00:30+05:30 is the 7th though it is the 6th in UTC, and "
        "reads the clock when given no `now`",
        reads,
    )

    missing = _OpsCase(None)
    missing.op("bundle.list_session_ids")
    missing.op("bundle.get_session", session_id=session)
    missing.op("bundle.get_swing", **where("1"))
    missing.op("bundle.attribute_unlabeled", stamps=True, session_id=session, player_id="aaron")
    missing.op("bundle.set_player", stamps=True, **where("1"), player_id="aaron")
    missing.op("bundle.set_club", stamps=True, **where("1"), club=seven)
    missing.op("bundle.set_mishit", stamps=True, **where("1"), verdict="confirmed")
    missing.op("bundle.delete_swing", **where("1"))
    record(
        "missing-root",
        "a root that does not exist reads as empty and every write on a swing in it is a None or "
        "False, never an error, and none of them creates the root "
        "(`test_missing_session_directory_is_empty_not_an_error`)",
        missing,
    )

    # ---------------------------------------------------------------------------- assignment

    roles = _OpsCase({})
    roles.upload(face, "swing1-face", player_id="aaron", club=seven)
    roles.upload(dtl, "swing1-dtl", player_id="aaron", club=seven)
    roles.upload(
        screen, "swing1-screen", filename="shot.jpg", content_type="image/jpeg",
        player_id="aaron", club=seven,
    )
    roles.upload(face, "swing2-face", player_id="aaron", club=seven)
    roles.op("bundle.get_session", session_id=session)
    record(
        "assign-roles",
        "the first upload into an empty root opens swing 1, a second role lands in it, the third "
        "completes it, and only then does a face-on open swing 2 "
        "(`test_two_different_roles_arrive_into_the_same_swing`, "
        "`test_second_swing_only_opens_once_first_is_complete`)",
        roles,
    )

    order = _OpsCase({})
    order.upload(screen, "screen", filename="shot.jpg", content_type="image/jpeg")
    order.upload(dtl, "dtl")
    order.upload(face, "face-on")
    record(
        "assign-arrival-order",
        "roles arriving in reverse still make one complete swing "
        "(`test_role_arrival_order_does_not_matter`)",
        order,
    )

    dedupe = _OpsCase({})
    dedupe.upload(face, "same-bytes", club=seven)
    dedupe.upload(face, "same-bytes", club=wedge, player_id="dave")
    dedupe.upload(dtl, "same-bytes")
    dedupe.upload(face, "same-bytes", session_id="2026-08-07")
    dedupe.op("bundle.get_session", session_id=session)
    record(
        "assign-dedupe",
        "the same bytes in the same role are deduped: the scratch file is removed, nothing is "
        "stamped, and the answer reports the stored club and golfer, not the requested ones "
        "(`test_duplicate_bytes_dedupe_instead_of_opening_a_new_swing`, "
        "`test_a_deduped_upload_reports_the_stored_club_not_the_requested_one`). The same bytes "
        "in another role, or in another session, are not a duplicate",
        dedupe,
    )

    newest = _OpsCase({})
    newest.upload(face, "first-recording")
    newest.upload(face, "different-recording")
    newest.upload(dtl, "dtl-a")
    newest.upload(dtl, "dtl-b")
    newest.upload(dtl, "dtl-c")
    record(
        "assign-newest-wins",
        "different bytes for a role a swing already has open the next swing, and a role both "
        "swings lack lands in the newest: the documented limitation "
        "(`test_different_bytes_reupload_of_a_filled_role_opens_a_new_swing`, "
        "`test_newest_wins_when_two_swings_are_missing_the_same_role`), then the older swing, "
        "then a third",
        newest,
    )

    target = _OpsCase({})
    target.upload(face, "first-recording", club=seven)
    target.upload(face, "corrected-recording", swing_id="1", club=wedge)
    target.upload(face, "corrected-recording", swing_id="1")
    target.upload(screen, "bad-photo", swing_id="1")
    target.upload(
        screen, "good-photo", filename="shot.jpg", content_type="image/jpeg", swing_id="1"
    )
    target.upload(screen, "photo-7", swing_id="7", player_id="aaron", club=wedge)
    target.upload(face, "next-face")
    target.upload(face, "after-that")
    target.upload(dtl, "named", swing_id="x")
    target.upload(face, "lands-in-x")
    target.op("bundle.get_session", session_id=session)
    record(
        "assign-explicit-target",
        "`swing_id` names the swing outright (the plan's finding 1): an existing slot is "
        "overwritten and the superseded file unlinked, the club already on the swing is kept, the "
        "same bytes again are re-placed rather than deduped, and an unknown id opens that swing, "
        "stamped from the call (`test_swing_id_repair_path_overwrites_the_original_slot`, "
        "`test_swing_id_override_replaces_a_role_in_place`). The next automatic upload then lands "
        "in the newest swing lacking its role (7), the one after opens 8 (one more than the "
        "highest numeric id), and a non-numeric `x` is newest and never counted",
        target,
    )

    corrupt = _OpsCase(
        {
            **_manifest_files(session, "1", (face, dtl, screen)),
            f"{session}/2/manifest.json": "{not json",
            f"{session}/2/face_on.0123456789ab.mov": "an orphaned clip",
            f"{session}/5/manifest.json": "{not json",
        }
    )
    corrupt.upload(face, "new-swing", club=seven)
    corrupt.upload(dtl, "named-over-corrupt", swing_id="5", club=wedge)
    corrupt.op("bundle.get_session", session_id=session)
    record(
        "assign-over-corrupt",
        "a corrupt manifest is invisible, so its swing number is reused: the next automatic swing "
        "is 2, written over directory 2's bad manifest, and the file already there stays. An "
        "explicit target with a corrupt manifest (5) is opened afresh the same way",
        corrupt,
    )

    golfer = _OpsCase({})
    golfer.upload(face, "a", player_id="aaron")
    golfer.upload(face, "b")
    golfer.upload(dtl, "c", player_id="aaron")
    golfer.upload(dtl, "d", player_id="dave")
    golfer.upload(face, "e", player_id="dave")
    golfer.op("bundle.get_session", session_id=session)
    record(
        "assign-golfer-stamping",
        "a new swing takes the call's golfer, an upload naming none leaves its swing anonymous, a "
        "later role labels that swing, an attributed swing is never restamped, and switching "
        "golfer stamps only what comes after "
        "(`test_a_new_swing_is_stamped_with_the_current_golfer` and the four tests after it)",
        golfer,
    )

    clubs = _OpsCase({})
    clubs.upload(face, "the-seven", club=seven)
    clubs.upload(face, "untagged")
    clubs.upload(dtl, "tags-it", club=seven)
    clubs.upload(dtl, "no-retag", club=wedge)
    clubs.upload(face, "the-wedge", club=wedge)
    clubs.op("bundle.get_session", session_id=session)
    record(
        "assign-club-stamping",
        "the club stamps by the golfer's rule: a new swing takes it, a later role tags an untagged "
        "swing, a tagged swing is never retagged, and switching club tags only what comes after "
        "(`test_a_new_swing_is_stamped_with_the_current_club` and the tests after it)",
        clubs,
    )

    names = _OpsCase({})
    for swing_id, (role, filename) in enumerate(
        [
            (face, "IMG_0001.MOV"),
            (face, "noext"),
            (dtl, "clip."),
            (screen, ".hidden"),
            (screen, "photo.tar.gz"),
            (face, "a b.MoV"),
            (screen, "IMG_2.HEIC"),
        ],
        start=1,
    ):
        names.upload(role, f"bytes-{swing_id}", filename=filename, swing_id=str(swing_id))
    names.op("bundle.get_session", session_id=session)
    record(
        "assign-content-filenames",
        "`content_filename` is `<role>.<sha256[:12]><suffix>`, the suffix `Path.suffix` of the "
        "original name as typed, else the role's default (`.mov`, or `.jpg` for the shot screen). "
        "`clip.` and `.hidden` have no suffix to `Path` (Rust's `Path::extension` reads `clip.` "
        "as an empty extension and would write a bare dot), and `photo.tar.gz` keeps only `.gz`",
        names,
    )

    other = _OpsCase(
        {
            **_manifest_files(session, "1", (face, dtl, screen), player_id="aaron", club=seven),
            **_manifest_files("2026-08-01", "5", (face,), directory=f"{session}/3", club=seven),
        }
    )
    other.op("bundle.get_session", session_id=session)
    other.upload(dtl, "follows-the-manifest")
    other.upload(face, "next-number")
    other.op("bundle.attribute_unlabeled", stamps=True, session_id=session, player_id="aaron")
    other.op("bundle.get_session", session_id=session)
    record(
        "manifest-names-another-swing",
        "a manifest is trusted over the directory it was found in. Directory 3 holds a manifest "
        "naming session 2026-08-01 and swing 5: a down-the-line upload into it is written to "
        "directory 5, and its answer names session 2026-08-01, the manifest's, while the dedupe "
        "answer would have named the call's. The next new swing is 6, one past the manifests' "
        "highest id. `attribute_unlabeled` saves each manifest to the directory its `swing_id` "
        "names, so directory 3's copy is written over directory 5 and then directory 5's own "
        "copy over that, and the answer names 5 twice",
        other,
    )

    # -------------------------------------------------------------------------------- repairs

    adopt = _OpsCase({})
    adopt.upload(face, "one")
    adopt.upload(face, "two", player_id="dave")
    adopt.upload(face, "three", club=seven)
    adopt.op("bundle.attribute_unlabeled", stamps=True, session_id=session, player_id="aaron")
    adopt.op("bundle.attribute_unlabeled", stamps=True, session_id=session, player_id="aaron")
    adopt.op("bundle.attribute_unlabeled", stamps=True, session_id="2026-08-09", player_id="aaron")
    adopt.op("bundle.get_session", session_id=session)
    record(
        "attribute-unlabeled",
        "the backfill adopts only the anonymous swings and returns their ids, is idempotent, "
        "touches no club, and on a missing session is [] "
        "(`test_attribute_unlabeled_adopts_only_the_anonymous_swings`, "
        "`test_attribute_unlabeled_is_idempotent`, "
        "`test_attribute_unlabeled_stamps_the_golfer_and_touches_no_club`)",
        adopt,
    )

    repairs = _OpsCase(
        {
            f"{session}/4/manifest.json": "{not json",
            "2026-08-07-aaron1/1/manifest.json": _PRE_M9_MANIFEST,
            "2026-08-07-aaron1/1/manifest.tmp": "a write that never finished",
        }
    )
    repairs.upload(face, "one", player_id="dave", club=wedge)
    repairs.upload(face, "two", player_id="dave", club=wedge)
    repairs.op("bundle.set_player", stamps=True, **where("1"), player_id="aaron")
    repairs.op("bundle.set_player", stamps=True, **where("99"), player_id="aaron")
    repairs.op("bundle.set_club", stamps=True, **where("2"), club=seven)
    repairs.op("bundle.set_club", stamps=True, **where("99"), club=seven)
    repairs.op("bundle.set_mishit", stamps=True, **where("1"), verdict="confirmed")
    repairs.op("bundle.set_mishit", stamps=True, **where("1"), verdict="cleared")
    repairs.op("bundle.set_mishit", stamps=True, **where("1"), verdict=None)
    repairs.op("bundle.set_mishit", stamps=True, **where("99"), verdict="confirmed")
    repairs.op("bundle.set_club", stamps=True, **where("4"), club=seven)
    repairs.op(
        "bundle.set_player", stamps=True, **where("1", "2026-08-07-aaron1"), player_id="aaron"
    )
    repairs.op("bundle.get_session", session_id=session)
    record(
        "repairs",
        "`set_player`, `set_club` and `set_mishit` overwrite one swing, leave its neighbours and "
        "each other's field alone, stamp `updated_at`, and answer None for a missing swing or a "
        "corrupt manifest, which stays as it was. `set_mishit` takes each verdict and None, which "
        "clears it. A manifest written before the three fields gains all three on its first "
        "save, through `manifest.tmp`, so a stale tmp file of that name is consumed",
        repairs,
    )

    deletes = _OpsCase(
        {
            **_manifest_files(session, "1", (face, dtl, screen), club=seven),
            f"{session}/1/analysis.json": "{}",
            f"{session}/1/analysis.state.json": "{}",
            f"{session}/1/face_on.keypoints.json": "{}",
            f"{session}/notes": "a file, not a swing",
        }
    )
    deletes.op("bundle.delete_swing", **where("1"))
    deletes.op("bundle.delete_swing", **where("1"))
    deletes.op("bundle.delete_swing", **where("99"))
    deletes.op("bundle.delete_swing", **where("notes"))
    deletes.op("bundle.get_session", session_id=session)
    record(
        "delete-swing",
        "a delete takes the whole directory, keypoints, analysis and state included, and leaves "
        "the session; deleting it again, a swing that never existed, or a file that is not a "
        "directory answers False (`test_delete_swing_removes_the_directory`, "
        "`test_delete_swing_on_a_missing_swing_returns_false`, "
        "`test_delete_swing_takes_everything_in_the_directory`)",
        deletes,
    )

    phantom = _OpsCase({})
    phantom.upload(face, "clip-1", club=seven)
    phantom.upload(screen, "bad-photo", club=seven)
    phantom.upload(screen, "good-photo", club=seven)
    phantom.upload(face, "clip-2", club=seven)
    phantom.op("bundle.delete_swing", **where("2"))
    phantom.op("bundle.get_session", session_id=session)
    record(
        "delete-phantom",
        "why `delete_swing` exists: a corrective shot-screen upload opens a phantom swing 2, the "
        "next swing's face-on is swallowed into it, and deleting it leaves swing 1 alone "
        "(`test_deleting_a_phantom_stops_it_swallowing_the_next_swing`)",
        phantom,
    )
    return cases


# ------------------------------------------------------------- storage: the bag, golfer and shots


def _storage_store_cases() -> dict[str, dict[str, Any]]:
    """The bag, golfer and shot stores' cases, and `slugify`'s table."""
    from datetime import UTC, datetime, timedelta, timezone

    from golf_coach.contracts.bag import Bag, BagEntry
    from golf_coach.contracts.club import ClubId
    from golf_coach.contracts.club_spec import SpecProvenance
    from golf_coach.contracts.golfer import Golfer, Handedness
    from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource

    stale = datetime(2020, 1, 1, tzinfo=UTC)
    cases: dict[str, dict[str, Any]] = {}

    def record(name: str, note: str, case: _OpsCase, source: str) -> None:
        assert name not in cases, f"two store cases are named {name}"
        cases[name] = _storage_vector(
            f"storage/stores/{name}", case.given(), note=note, source=source, run=_run_ops
        )

    def entry(club: str, **fields: Any) -> dict[str, Any]:
        """A declaration as a caller writes one: `recorded_at` supplied and about to be ignored
        (`test_bag_store.py::_entry`), with only the fields it sets."""
        built = BagEntry(club=ClubId(club), recorded_at=fields.pop("recorded_at", stale), **fields)
        return built.model_dump(mode="json", exclude_defaults=True)

    def bag_text(player_id: str, entries: dict[str, Any], **fields: Any) -> str:
        bag = Bag(
            player_id=player_id,
            entries={ClubId(club): BagEntry.model_validate(e) for club, e in entries.items()},
            updated_at=fields.pop("updated_at", stale),
            **fields,
        )
        return bag.model_dump_json(indent=2)

    def golfer_text(player_id: str, name: str, handedness: str = "right") -> str:
        return Golfer(
            player_id=player_id,
            display_name=name,
            handedness=Handedness(handedness),
            created_at=datetime(2026, 8, 12, 9, 0, tzinfo=UTC),
        ).model_dump_json(indent=2)

    def shot(shot_id: str, when: datetime, *, digest: str | None, **fields: Any) -> dict[str, Any]:
        """`test_screen_store.py::_shot`: a parsed screen shot, keyed by `digest` if it has one."""
        provenance = fields.pop(
            "provenance",
            ShotProvenance(
                device="hd_golf", parse_confidence=0.95, needs_review=False, image_sha256=digest
            ),
        )
        built = ShotData(
            shot_id=shot_id,
            session_id=fields.pop("session_id", "range"),
            timestamp=when,
            source=ShotSource.SCREEN,
            carry_distance=fields.pop("carry_distance", 128.1),
            provenance=provenance,
            **fields,
        )
        return built.model_dump(mode="json", exclude_defaults=True)

    bag_source = "tests/storage/test_bag_store.py, as store calls"
    golfer_source = "tests/storage/test_golfer_store.py, as store calls"
    shot_source = "tests/launch_monitor/test_screen_store.py, as store calls"

    # ------------------------------------------------------------------------------ the bag

    seven = {
        "club": "7i",
        "make": "Titleist",
        "loft_deg": 34.0,
        "recorded_at": "2026-08-01T10:00:00Z",
    }
    reads = _OpsCase(
        {
            "aaron.bag.json": bag_text("aaron", {"7i": seven}),
            "bob.bag.json": "{not json",
            "carol.bag.json": json.dumps(
                {
                    "player_id": "carol",
                    "entries": {"7i": {**seven, "club": "pw"}},
                    "updated_at": "2026-08-01T10:00:00Z",
                }
            ),
            "dave.bag.json": json.dumps({"player_id": "dave", "entries": {}}),
            "erin.bag.json": json.dumps(
                {"player_id": "Erin", "entries": {}, "updated_at": "2026-08-01T10:00:00Z"}
            ),
            "frank.bag.json": json.dumps(
                {
                    "player_id": "frank",
                    "entries": {"7i": {**seven, "retired_at": "2026-08-02T10:00:00Z"}},
                    "updated_at": "2026-08-01T10:00:00Z",
                }
            ),
        }
    )
    for player in ("aaron", "bob", "carol", "dave", "erin", "frank", "nobody"):
        reads.op("bag.get", player_id=player)
    given_bag = json.loads(bag_text("gina", {"pw": entry("pw", loft_deg=46.0)}))
    reads.op("bag.save", bag=given_bag)
    reads.op("bag.get", player_id="gina")
    reads.op("bag.save", bag={**given_bag, "player_id": "aaron"})
    reads.op("bag.get", player_id="aaron")
    record(
        "bag-reads",
        "`get` is tolerant: no bag, bad JSON, a slot holding another club (`_keys_match_entries`), "
        "a missing `updated_at`, a `player_id` that is not a slug and a live entry carrying "
        "`retired_at` all read as None (`test_a_golfer_with_no_bag_reads_as_none`, "
        "`test_a_corrupt_bag_reads_as_none`). `save` writes the bag it was given, overwriting, and "
        "stamps nothing, so it reads no clock (`test_save_writes_the_bag_it_was_given`)",
        reads,
        bag_source,
    )

    upsert = _OpsCase({})
    titleist = {"make": "Titleist", "loft_deg": 34.0}
    upsert.op("bag.set_entry", stamps=True, player_id="aaron", entry=entry("7i", **titleist))
    upsert.op("bag.set_entry", stamps=True, player_id="aaron", entry=entry("7i", **titleist))
    upsert.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry(
            "7i",
            **titleist,
            provenance=SpecProvenance(source="llm:claude-opus-5", retrieved_at=stale),
        ),
    )
    upsert.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry("7i", **titleist, shaft_model="Modus 105"),
    )
    upsert.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry("7i", **titleist, shaft_model="Project X LZ"),
    )
    upsert.op("bag.set_entry", stamps=True, player_id="aaron", entry=entry("pw", make="Ping"))
    upsert.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry("pw", make="Ping", loft_deg=46.0),
    )
    upsert.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry(
            "driver",
            make="Callaway",
            model="Paradym",
            model_year=2023,
            loft_deg=9.0,
            adjustable_hosel=True,
            loft_range_deg=(7.25, 10.75),
            shaft_material="graphite",
            shaft_flex="x_stiff",
            shaft_weight_g=62.5,
            usga_conforming=True,
            provenance=SpecProvenance(
                source="catalogue",
                retrieved_at=datetime(2026, 8, 31, 8, 15, 30, 250000, tzinfo=UTC),
                notes="published",
            ),
        ),
    )
    upsert.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry(
            "7i", make="Ping", loft_deg=32.0, retired_at=datetime(2021, 1, 1, tzinfo=UTC)
        ),
    )
    upsert.op("bag.get", player_id="aaron")
    record(
        "bag-set-entry",
        "`set_entry`'s three cases: an empty slot is installed stamped `now`, the caller's "
        "`recorded_at` discarded; the same club again, or the same club with only a fresh "
        "`provenance`, is returned untouched and not written (`same_club_as`); a different club, "
        "a re-shaft or a first measured loft among them, retires the old entry stamped `now` onto "
        "the shelf. A caller's `retired_at` is cleared on install "
        "(`test_re_saving_an_unchanged_club_does_not_move_recorded_at` and the upsert tests "
        "after it)",
        upsert,
        bag_source,
    )

    shelf = _OpsCase({})
    shelf.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry("3w", make="TaylorMade", loft_deg=15.0),
    )
    shelf.op("bag.set_entry", stamps=True, player_id="aaron", entry=entry("7i", **titleist))
    shelf.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry("7i", make="Ping", loft_deg=32.0),
    )
    shelf.op("bag.remove_entry", stamps=True, player_id="aaron", club="3w")
    shelf.op("bag.remove_entry", stamps=True, player_id="aaron", club="3w")
    shelf.op("bag.remove_entry", stamps=True, player_id="aaron", club="driver")
    shelf.op("bag.remove_entry", stamps=True, player_id="nobody", club="driver")
    shelf.op("bag.restore_entry", stamps=True, player_id="aaron", club="7i")
    shelf.op("bag.restore_entry", stamps=True, player_id="aaron", club="3w")
    shelf.op("bag.restore_entry", stamps=True, player_id="aaron", club="driver")
    shelf.op("bag.restore_entry", stamps=True, player_id="nobody", club="7i")
    shelf.op("bag.restore_entry", stamps=True, player_id="aaron", club="7i")
    shelf.op("bag.remove_entry", stamps=True, player_id="aaron", club="3w")
    shelf.op(
        "bag.set_entry",
        stamps=True,
        player_id="aaron",
        entry=entry("3w", make="TaylorMade", loft_deg=15.0),
    )
    shelf.op("bag.restore_entry", stamps=True, player_id="aaron", club="3w")
    shelf.op("bag.get", player_id="aaron")
    record(
        "bag-remove-restore",
        "`remove_entry` shelves the club stamped `now`, and answers None without writing for an "
        "empty slot or no bag; `restore_entry` copies the slot's newest stint back, freshly "
        "stamped, and leaves the stint on the append-only shelf, so out and back is two stints; "
        "None for a slot that never held another or no bag. A restore whose stint is the club "
        "already in the slot is `set_entry`'s same-club case: untouched and not written "
        "(`test_removing_a_club_shelves_it` and the restore tests after it)",
        shelf,
        bag_source,
    )

    guard = _OpsCase(
        {
            "aaron.bag.json": "{not json",
            "carol.bag.json": json.dumps(
                {
                    "player_id": "carol",
                    "entries": {"7i": {**seven, "club": "pw"}},
                    "updated_at": "2026-08-01T10:00:00Z",
                }
            ),
        }
    )
    guard.op("bag.set_entry", stamps=True, player_id="aaron", entry=entry("driver"))
    guard.op("bag.remove_entry", stamps=True, player_id="aaron", club="7i")
    guard.op("bag.restore_entry", stamps=True, player_id="aaron", club="7i")
    guard.op("bag.set_entry", stamps=True, player_id="carol", entry=entry("driver"))
    guard.op("bag.get", player_id="aaron")
    guard.op("bag.set_entry", stamps=True, player_id="../aaron", entry=entry("driver"))
    guard.op("bag.set_entry", stamps=True, player_id="Dave Smith", entry=entry("driver"))
    record(
        "bag-write-guard",
        "a bag that exists and cannot be read, as JSON or as a `Bag`, is never written over: each "
        "mutator raises the store's `ValueError` and the file is unchanged, while `get` still "
        "reads it as None (`test_a_corrupt_bag_is_never_written_over`). A `player_id` that is not "
        "a slug is refused by `Bag`'s validator before a path is made of it "
        "(`test_a_player_id_that_is_not_a_slug_never_becomes_a_filename`)",
        guard,
        bag_source,
    )

    shared = _OpsCase({})
    shared.op("golfer.get_or_create", stamps=True, name="Aaron", handedness="right")
    shared.op("bag.set_entry", stamps=True, player_id="aaron", entry=entry("7i", loft_deg=34.0))
    shared.op("golfer.list_all")
    shared.op("golfer.get", player_id="aaron")
    shared.op("bag.get", player_id="aaron")
    record(
        "bag-beside-golfer",
        "the golfer and the bag share a directory without seeing each other: `list_all` globs "
        "`*.golfer.json` and so never meets `aaron.bag.json` "
        "(`test_bags_and_golfers_share_a_directory_without_seeing_each_other`)",
        shared,
        bag_source,
    )

    # ------------------------------------------------------------------------------ golfers

    create = _OpsCase({})
    for name, handedness in [
        ("Aaron", "right"),
        ("Aaron", "right"),
        ("  aaron ", "right"),
        ("aaron", "left"),
        ("María", "right"),
        ("Maria", "left"),
        ("Björn Éamon O'Brien", "left"),
        (" Zoë ", "left"),
        ("\x1cBob\x1f", "right"),
        ("!!!", "right"),
        ("", "right"),
        ("   ", "right"),
        ("​", "right"),
        ("'", "right"),
    ]:
        create.op("golfer.get_or_create", stamps=True, name=name, handedness=handedness)
    create.op("golfer.list_all")
    record(
        "golfer-get-or-create",
        "`get_or_create` is read-biased: a name that slugs to a known golfer returns the stored "
        "record, its handedness and original display name untouched, so a retyped or accented "
        "spelling is the same golfer (`test_a_retyped_name_resolves_to_the_existing_golfer`, "
        "`test_handedness_of_a_known_golfer_is_never_overwritten`, "
        "`test_accented_and_unaccented_spellings_are_the_same_golfer`). The display name is "
        "`name.strip()`, which strips what `str.isspace` calls whitespace: NBSP and EM SPACE, and "
        "the separators \\x1c and \\x1f, which Rust's `trim` keeps. A name that slugs to nothing "
        "raises, its `repr` in the message (`'\\u200b'`, and `\"'\"` in double quotes)",
        create,
        golfer_source,
    )

    golfers = _OpsCase(
        {
            "alice.golfer.json": golfer_text("alice", "same"),
            "bob.golfer.json": golfer_text("bob", "Same", "left"),
            "carol.golfer.json": golfer_text("carol", "SAME"),
            "zed.golfer.json": golfer_text("zed", "Émile"),
            "inci.golfer.json": golfer_text("inci", "İnci"),
            "inga.golfer.json": golfer_text("inga", "Inga"),
            "mismatch.golfer.json": golfer_text("aaron", "Aaron"),
            "corrupt.golfer.json": "{not json",
            "badid.golfer.json": json.dumps(
                {
                    "player_id": "Bad Id",
                    "display_name": "Bad",
                    "handedness": "right",
                    "created_at": "2026-08-12T09:00:00Z",
                }
            ),
            "aaron.bag.json": bag_text("aaron", {}),
            "notes.txt": "not a golfer",
            "stale.golfer.tmp": "a write that never finished",
        }
    )
    for player in ("alice", "mismatch", "corrupt", "badid", "nobody"):
        golfers.op("golfer.get", player_id=player)
    golfers.op("golfer.list_all")
    golfers.op("golfer.get_or_create", stamps=True, name="Corrupt", handedness="left")
    golfers.op("golfer.get", player_id="corrupt")
    record(
        "golfer-reads",
        "`get` reads the record the file holds, so `mismatch.golfer.json` answers golfer `aaron`, "
        "and an unreadable or invalid record is None (`test_unknown_and_corrupt_records_read_as_"
        "absent`). `list_all` skips those, ignores what is not `*.golfer.json`, and sorts on "
        "`display_name.lower()` by code point: a capital dotted I (U+0130) lowers to i + U+0307, "
        "so `inci`'s name sorts after `Inga`, `Émile` after every ASCII name, and three names "
        "equal once lowered keep the files' order (`test_list_all_is_sorted_by_display_name`). "
        "`get_or_create` over a corrupt record creates a fresh golfer and writes over it",
        golfers,
        golfer_source,
    )

    nowhere = _OpsCase(None)
    nowhere.op("golfer.list_all")
    nowhere.op("golfer.get", player_id="aaron")
    nowhere.op("golfer.get_or_create", stamps=True, name="Aaron", handedness="right")
    record(
        "golfer-missing-root",
        "a golfer directory that does not exist lists nothing and reads None, and the first "
        "golfer created makes it",
        nowhere,
        golfer_source,
    )

    # ------------------------------------------------------------------------------ shots

    noon = datetime(2026, 8, 4, 12, 0, tzinfo=UTC)
    first, second = _hash_text("photo"), _hash_text("a different photo")
    shots = _OpsCase({})
    shots.op("shot.put", shot=shot("s1", noon, digest=first))
    shots.op("shot.has", key=first)
    shots.op("shot.get", key=first)
    shots.op("shot.has", key=second)
    shots.op("shot.get", key=second)
    shots.op(
        "shot.put",
        shot=shot("2026-08-04 12:00/swing#1", noon + timedelta(minutes=5), digest=None,
                  provenance=None),
    )
    shots.op("shot.has", key="2026-08-04 12:00/swing#1")
    shots.op("shot.put", shot=shot("s3", noon + timedelta(minutes=10), digest=None))
    shots.op("shot.put", shot=shot("s4", noon + timedelta(minutes=15), digest=""))
    shots.op("shot.put", shot=shot("naïve…", noon + timedelta(minutes=20), digest=None))
    shots.op("shot.all")
    shots.op("shot.put", shot=shot("s1", noon, digest=first, carry_distance=131.4))
    shots.op("shot.get", key=first)
    record(
        "shot-store",
        "the shot store is content-addressed: `put` keys a shot by its photo's sha256 and falls "
        "back to `shot_id` when there is no provenance, no hash or an empty one, each "
        "`[^A-Za-z0-9._-]+` run of the key becoming one `-`, non-ASCII included; it answers the "
        "path written, and a second `put` of a key writes over the first "
        "(`test_round_trip_through_the_store`, `test_reimporting_the_same_image_hits_the_cache`)",
        shots,
        shot_source,
    )

    def shot_text(shot_id: str, when: datetime) -> str:
        return ShotData.model_validate(shot(shot_id, when, digest=None)).model_dump_json(indent=2)

    india = timezone(timedelta(hours=5, minutes=30))
    eastern = timezone(timedelta(hours=-5))
    ordered = _OpsCase(
        {
            "a.shot.json": shot_text("a", noon),
            "b.shot.json": shot_text("b", noon),
            "c.shot.json": shot_text("c", datetime(2026, 8, 4, 17, 0, tzinfo=india)),
            "d.shot.json": shot_text("d", datetime(2026, 8, 4, 8, 0, tzinfo=eastern)),
            "e.shot.json": "{not json",
            "f.shot.json": json.dumps({"shot_id": "f", "session_id": "range"}),
            "g.json": shot_text("g", noon),
        }
    )
    ordered.op("shot.all")
    ordered.op("shot.get", key="e")
    ordered.op("shot.get", key="f")
    ordered.op("shot.get", key="g")
    ordered.op("shot.has", key="e")
    record(
        "shot-all-order",
        "`all` is newest first by instant, not by spelling: 08:00-05:00 is 13:00Z and leads, "
        "17:00+05:30 is 11:30Z and trails. Two shots at one instant keep the files' sorted order "
        "(a stable sort, reversed by `reverse=True` and not by reversing the list). Unreadable "
        "files are skipped (`test_unreadable_files_are_skipped_not_fatal`), but `get` on one "
        "raises, unlike every other reader here; `g.json` is not a `.shot.json` and is never seen",
        ordered,
        shot_source,
    )

    no_shots = _OpsCase(None)
    no_shots.op("shot.all")
    no_shots.op("shot.has", key=first)
    no_shots.op("shot.get", key=first)
    no_shots.op("shot.put", shot=shot("s1", noon, digest=first))
    record(
        "shot-missing-root",
        "a shot directory that does not exist is empty, not an error "
        "(`test_missing_store_directory_is_empty_not_an_error`), and the first `put` makes it",
        no_shots,
        shot_source,
    )

    # ------------------------------------------------------------------------------ slugify

    table = _OpsCase(None)
    for name in [
        # `test_slugify_folds_names_to_a_stable_id`, in its order.
        "Aaron", "  Aaron  ", "AARON", "Aaron Sierra", "Aaron  Sierra", "O'Brien", "Player 2",
        "!!!", "", "María", "Maria", "Björn", "Éamon",
        # The edges a port meets: hyphen runs and ends, a lowercase that leaves ASCII or lands in
        # it (`İ`, the Kelvin sign), letters NFD cannot take apart (`ß`, `ø`, a ligature, a
        # titlecase digraph, a roman numeral), full-width forms, an already-decomposed accent and
        # a leading one, and two marks with combining class 0 that a general-category test would
        # strip (U+0903, U+20DD) but `unicodedata.combining` keeps for `_NON_SLUG` to replace.
        "Aaron ", "--aaron--", "a--b", "a.b_c", "José-María Ruiz", "Nguyễn", "123", "--",
        "İnci", "Kelvin", "Straße", "Søren", "ﬁne", "ǅ", "Ⅻ",
        "Player ２", "Ａaron", "é", "́abc", "aःb", "a⃝b",
        "A\tB\nC", " Zoë ", "\x1cBob\x1f", "Ωmega",
    ]:
        table.op("slugify", name=name)
    record(
        "slugify",
        "`contracts/golfer.py::slugify` over `test_slugify_folds_names_to_a_stable_id`'s table and "
        "the Unicode edges a port meets: `strip`, `lower`, NFD, drop what "
        "`unicodedata.combining` calls combining (canonical class non-zero), then every "
        "`[^a-z0-9]+` run to one `-` and the ends stripped of `-`",
        table,
        "tests/storage/test_golfer_store.py::test_slugify_folds_names_to_a_stable_id",
    )
    return cases


def _hash_text(text: str) -> str:
    """`launch_monitor/screen/store.py::hash_image` of a text's UTF-8 bytes: a shot store key."""
    from golf_coach.launch_monitor.screen.store import hash_image

    return hash_image(text.encode("utf-8"))


# ========================================================== career: the aggregates and the reports
#
# **The career family records what one golfer's history supports saying** (M36 P3, the M36 plan's
# calls 2 and 3): `analysis/{baseline,dispersion,comparison,club_profile}.py` over a corpus, and the
# text the five career scripts print from what those build. `regenerate --career-once` records it
# once and refuses a second run, on the storage family's precedent. From there `crates/analysis`
# (the aggregates) and `crates/core` (the reports) are the implementation, and `golf-core rerecord`
# the only writer, under `CAREER_VERSION`, the key this family shares with the storage family (the
# plan's decision 8).
#
# **A case's input is a corpus, not a tree.** `input.corpus` is a `CareerCorpus` as
# `model_dump(mode="json")` writes it, so the aggregates are gated apart from the reader: a port can
# be wrong about `read_corpus` and right about `build_baseline`, and the two families say which.
# `input.bag` is a `Bag` or null, `input.display_name` the name a report's header prints, and
# `input.versions` the storage family's pair, because the corpus report prints the installed engine
# (`_run_career` refuses any pair but frozen Python's own, as `_run_corpus` does). `input.clubs` is
# what `club_profile.py --club` is recorded for: every club in the profile, then the first `ClubId`
# that is not, whose report is the "never hit and not in the bag" sentence. The plan's call 2 had a
# single `club`, which could not say which clubs a case's reports cover.
#
# **Three kinds of case.** *Built* (`synthetic/<name>`, `_career_built`): `CorpusSwing`s
# constructed directly, as `tests/analysis/` constructs them, chosen for the aggregates' rows, and
# checked at build time to land where their note says. Each corpus goes through `narrowed_to()`
# with no arguments, which fills `metric_counts` and the counters a reader would have.
# *Adopted* (`synthetic/storage-<name>`, `_career_adopted`): every synthetic storage corpus case's
# `expected.corpus`, read from the committed family, so every corpus a reader actually produces
# (exclusions, re-uploads, an outdated engine, a stranger's swings) is rendered by every report.
# *Real* (`real/aaron`, `_career_real`): the real storage vector's `expected.corpus`, which must
# equal `read_corpus` over `data/` or the recorder raises (the plan's call 2), with the golfer's
# own bag and name.
#
# **`expected` is `{baseline, dispersion, standing, bag_profile, properties, reports}`.** The four
# aggregates as `model_dump(mode="json")` writes them; `properties`, what they derive that
# `model_dump` drops and a report prints (P1's corpus `properties`, one family on); and the text.
#
# **The reports are each script's own function**, loaded by path and captured with
# `redirect_stdout` (the plan's call 3), so the text stays gated after M29 deletes the scripts:
# `<script>` and `<script>_verbose` for the four `_report`s, `club_profile_<club>` once per
# `input.clubs`, and `flag_mishit_list`. `flag_mishit._list` reads stores, so it runs with
# `read_corpus` answering the case's corpus and a registry holding the case's golfer alone, which
# is the renderer over that corpus and nothing else; over the real case the recorder also runs it
# over the real trees, materialised, and raises unless the two print the same. A script's `main`
# adds the golfer loop and a trailing blank line around these, which a verb reproduces and P16's
# parity run checks.

#: The career family's root.
_CAREER = VECTORS / "career"

#: The golfer records and bags `data/` holds, beside the sessions they swung.
GOLFERS = REPO / "data" / "processed" / "golfers"

#: The scripts whose report text the family records, by file stem.
_CAREER_SCRIPTS = (
    "career_corpus",
    "career_baseline",
    "career_dispersion",
    "club_profile",
    "flag_mishit",
)

#: Each script as a module, loaded by path once per process (`_career_scripts`).
_LOADED_SCRIPTS: dict[str, Any] = {}

#: What every built case's measurements are, by name: unit and `Measurement.source`. The units are
#: the engine's own, and three of them (`rpm`, `sd_units`, `furlongs`) are outside the scripts'
#: precision table, so they print at its three-decimal default.
_CAREER_KINDS: dict[str, tuple[str, str]] = {
    "head_sway_norm": ("shoulder_widths", "pose:face_on"),
    "hip_sway_norm": ("shoulder_widths", "pose:face_on"),
    "hip_shift_at_top_norm": ("shoulder_widths", "pose:face_on"),
    "finish_balance_norm": ("shoulder_widths", "pose:face_on"),
    "head_hip_offset_impact_norm": ("shoulder_widths", "pose:face_on"),
    "head_hip_gain_norm": ("shoulder_widths", "pose:face_on"),
    "hand_height_norm": ("shoulder_widths", "pose:face_on"),
    "pivot_hip_axis_drift_norm": ("shoulder_widths", "pose:face_on"),
    "pivot_hip_axis_drift_norm_dtl": ("shoulder_widths", "pose:down_the_line"),
    "tempo_ratio": ("ratio", "pose:face_on"),
    "backswing_ms": ("ms", "pose:face_on"),
    "downswing_ms": ("ms", "pose:face_on"),
    "face_to_path_deg": ("degrees", "launch_monitor:hd_golf"),
    "start_line_deg": ("degrees", "launch_monitor:hd_golf"),
    "start_line_offline_yds": ("yards", "launch_monitor:hd_golf"),
    "carry_distance_yds": ("yards", "launch_monitor:hd_golf"),
    "total_distance_yds": ("yards", "launch_monitor:hd_golf"),
    "ball_speed_mph": ("mph", "launch_monitor:hd_golf"),
    "launch_angle_deg": ("degrees", "launch_monitor:hd_golf"),
    "flight_carry_yds": ("yards", "model:flight_v1"),
    "flight_spin_rpm": ("rpm", "model:flight_v1"),
    "tour_joint_distance": ("sd_units", "population:golfdb"),
    "mystery_reach_norm": ("furlongs", "mystery:sensor"),
}


def build_career(*, real: bool = True) -> list[tuple[Path, dict[str, Any]]]:
    """The career family, as (path, payload) pairs, built in full before anything is returned.

    Writes nothing. The adopted and real cases read the committed storage family, so
    `--storage-once` has to have run first. `real=False` leaves out the case read from `data/`,
    which is what lets the dry-run pin in `tests/test_conformance.py` run with no captures.
    """
    synthetic = _CAREER / "synthetic"
    out = [(synthetic / f"{name}.json", v) for name, v in _career_built().items()]
    out += [(synthetic / f"{name}.json", v) for name, v in _career_adopted().items()]
    if real:
        out.append((_CAREER / "real" / f"{_REAL_PLAYER}{_REAL_SUFFIX}", _career_real()))
    return out


def _run_career(given: dict[str, Any]) -> dict[str, Any]:
    """One career case's input through frozen Python: the definition a port reproduces.

    The four aggregates over `input.corpus` (the bag profile with `input.bag` beside it), the values
    they derive, and the scripts' report text over them.
    """
    from golf_coach.analysis.baseline import build_baseline
    from golf_coach.analysis.club_profile import build_bag_profile
    from golf_coach.analysis.comparison import build_standing
    from golf_coach.analysis.dispersion import build_dispersion
    from golf_coach.contracts.bag import Bag
    from golf_coach.contracts.career import CareerCorpus

    frozen = {"installed": ANALYSIS_VERSION, "comparable_from": ANALYSIS_VERSION}
    if given["versions"] != frozen:
        raise ValueError(
            f"frozen Python's corpus report prints its own engine as the installed one, so it "
            f"answers only under {frozen}; {given['versions']} is a question for Rust's"
        )
    corpus = CareerCorpus.model_validate(given["corpus"])
    bag = None if given["bag"] is None else Bag.model_validate(given["bag"])
    baseline = build_baseline(corpus)
    dispersion = build_dispersion(corpus)
    standing = build_standing(corpus)
    profile = build_bag_profile(corpus, bag)
    return {
        "baseline": baseline.model_dump(mode="json"),
        "dispersion": dispersion.model_dump(mode="json"),
        "standing": standing.model_dump(mode="json"),
        "bag_profile": profile.model_dump(mode="json"),
        "properties": {
            "baseline": {
                "claims_ready": baseline.claims_ready,
                "nothing_sayable": baseline.nothing_sayable,
            },
            "dispersion": {
                "patterns_established": dispersion.patterns_established,
                "nothing_established": dispersion.nothing_established,
            },
            "standing": {
                "placements": standing.placements,
                "nothing_placed": standing.nothing_placed,
            },
            "bag_profile": {
                "clubs_used": [one.club.value for one in profile.clubs_used],
                "clubs_declared": [one.club.value for one in profile.clubs_declared],
                "categories": {one.club.value: one.category.value for one in profile.clubs},
            },
        },
        "reports": _career_reports(given, corpus, baseline, dispersion, standing, profile),
    }


def _career_reports(
    given: dict[str, Any],
    corpus: Any,
    baseline: Any,
    dispersion: Any,
    standing: Any,
    profile: Any,
) -> dict[str, str]:
    """Each script's report over one case, keyed as the section header says."""
    from golf_coach.contracts.club import ClubId

    scripts = _career_scripts()
    name = given["display_name"]
    reports: dict[str, str] = {}
    for verbose in (False, True):
        tail = "_verbose" if verbose else ""
        reports[f"career_corpus{tail}"] = _printed(
            scripts["career_corpus"]._report, corpus, name, verbose=verbose
        )
        reports[f"career_baseline{tail}"] = _printed(
            scripts["career_baseline"]._report, baseline, standing, name, verbose=verbose
        )
        reports[f"career_dispersion{tail}"] = _printed(
            scripts["career_dispersion"]._report, dispersion, name, verbose=verbose
        )
        reports[f"club_profile{tail}"] = _printed(
            scripts["club_profile"]._report, profile, name, verbose=verbose, club=None
        )
    for club in given["clubs"]:
        reports[f"club_profile_{club}"] = _printed(
            scripts["club_profile"]._report, profile, name, verbose=False, club=ClubId(club)
        )
    reports["flag_mishit_list"] = _mishit_listing(scripts["flag_mishit"], corpus)
    return reports


def _career_scripts() -> dict[str, Any]:
    """The five scripts as modules, loaded by path as `_load_conftest` loads a conftest: `scripts/`
    is not a package, and an import by name would depend on whoever put it on `sys.path`."""
    if not _LOADED_SCRIPTS:
        for stem in _CAREER_SCRIPTS:
            _LOADED_SCRIPTS[stem] = _load_conftest(
                REPO / "scripts" / f"{stem}.py", f"_conformance_script_{stem}"
            )
    return _LOADED_SCRIPTS


def _printed(fn: Callable[..., Any], *args: Any, **kwargs: Any) -> str:
    """What `fn` prints, as one string. It must return as a report (`None`) or a listing (0) does,
    so a refusal path a case did not mean to reach stops the build."""
    import io

    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        returned = fn(*args, **kwargs)
    assert returned in (None, 0), f"{fn.__name__} returned {returned!r}, not a report"
    return buffer.getvalue()


def _mishit_listing(module: Any, corpus: Any) -> str:
    """`flag_mishit --list` for the case's golfer alone, `read_corpus` answering the case's corpus.

    `_list` takes a golfer registry and reads each golfer's corpus from the settings' sessions
    directory. Both are answered from the case here, so what it prints is the renderer over this
    corpus; `_career_real` holds that to `_list` over the real trees.
    """
    from types import SimpleNamespace

    golfer = SimpleNamespace(player_id=corpus.player_id)

    def read_corpus(sessions_dir: Any, player_id: str) -> Any:
        assert player_id == corpus.player_id, f"the listing asked for {player_id}"
        return corpus

    registry = SimpleNamespace(
        get=lambda player_id: golfer if player_id == corpus.player_id else None,
        list_all=lambda: [golfer],
    )
    with _patched(module, read_corpus=read_corpus):
        return _printed(module._list, None, registry, None)


@contextlib.contextmanager
def _patched(module: Any, **names: Any) -> Iterator[None]:
    """`module`'s `names` answered by the values given, for the block, and restored after it."""
    saved = {name: getattr(module, name) for name in names}
    for name, value in names.items():
        setattr(module, name, value)
    try:
        yield
    finally:
        for name, value in saved.items():
            setattr(module, name, value)


def _career_vector(name: str, given: dict[str, Any], *, note: str, source: str) -> dict[str, Any]:
    """Serialize the input, run it through `_run_career`, and pair the two (`_vector`'s rule).

    `provenance.analysis_version` is the engine the corpus report names as installed; a career
    vector does not age on it.
    """
    import pydantic

    given_json = json.loads(json.dumps(given))
    return {
        "id": f"career/{name}",
        "career_version": _FROZEN_CAREER,
        "provenance": {
            "kind": "career",
            "oracle": "python",
            "note": note,
            "source": source,
            "analysis_version": ANALYSIS_VERSION,
            "python_version": _python_version(),
            "pydantic_version": pydantic.VERSION,
            "recorded_by": "scripts/conformance_vectors.py::_run_career",
        },
        "input": given_json,
        "expected": _run_career(given_json),
    }


def _career_given(corpus: Any, *, bag: Any = None, display_name: str = "Aaron") -> dict[str, Any]:
    """A case's input: the corpus and bag as dumped, and the clubs `--club` is recorded for."""
    from golf_coach.contracts.club import ClubId

    hit = {swing.club for swing in corpus.swings if swing.club is not None}
    declared = set(bag.entries) if bag is not None else set()
    profiled = [club for club in ClubId if club in hit or club in declared]
    absent = next(club for club in ClubId if club not in profiled)
    return {
        "corpus": corpus.model_dump(mode="json"),
        "bag": None if bag is None else bag.model_dump(mode="json"),
        "display_name": display_name,
        "versions": {"installed": ANALYSIS_VERSION, "comparable_from": ANALYSIS_VERSION},
        "clubs": [club.value for club in (*profiled, absent)],
    }


def _digest(label: str) -> str:
    """A sha256 standing in for a clip's or a photo's: the shape the reports print 12 of."""
    return hashlib.sha256(label.encode("utf-8")).hexdigest()


# ------------------------------------------------------------------------ career: the built cases


def _career_built() -> dict[str, dict[str, Any]]:
    """The cases built here, one or more per row of the M36 plan's P3 list.

    Each is checked at build time to land where its note says (a floor shut or open, a pattern, a
    standing, a caveat's form), so a change to a table under one stops the build rather than
    recording a case that no longer pins what it is named for.
    """
    from datetime import UTC, datetime, timedelta, timezone

    from golf_coach.analysis.baseline import build_baseline
    from golf_coach.analysis.benchmarks import load_distribution
    from golf_coach.analysis.club_profile import build_bag_profile
    from golf_coach.analysis.comparison import build_standing
    from golf_coach.analysis.dispersion import build_dispersion
    from golf_coach.contracts.bag import Bag, BagEntry
    from golf_coach.contracts.baseline import BaselineClaim, minimum_n
    from golf_coach.contracts.career import CareerCorpus, CorpusSwing, ExcludedSwing
    from golf_coach.contracts.career import ExclusionReason as Reason
    from golf_coach.contracts.club import ClubId
    from golf_coach.contracts.club_spec import SpecProvenance
    from golf_coach.contracts.comparison import Standing
    from golf_coach.contracts.dispersion import SCATTER_ONLY_READING, DispersionPattern, Finding
    from golf_coach.contracts.mishit import MishitVerdict
    from golf_coach.contracts.swing import Measurement

    weeks = ("2026-08-03", "2026-08-10", "2026-08-17")
    seven, driver, five, nine = (
        ClubId.SEVEN_IRON,
        ClubId.DRIVER,
        ClubId.FIVE_IRON,
        ClubId.NINE_IRON,
    )

    def on(session: str, minute: int = 0, *, hour: int = 12, tz: Any = UTC) -> datetime:
        year, month, day = (int(part) for part in session.split("-"))
        return datetime(year, month, day, hour, tzinfo=tz) + timedelta(minutes=minute)

    def swing(
        index: int,
        values: dict[str, float] | None = None,
        *,
        session: str,
        at: datetime | None = None,
        club: ClubId | None = None,
        clip: int | str | None = None,
        photo: Any = True,
        **fields: Any,
    ) -> CorpusSwing:
        """One distinct swing: its own clip and photo unless told to share one (or have none)."""
        fields.setdefault("analyzed", True)
        fields.setdefault("analysis_version", ANALYSIS_VERSION if fields["analyzed"] else 0)
        return CorpusSwing(
            player_id=_REAL_PLAYER,
            session_id=session,
            swing_id=str(index),
            captured_at=at or on(session, index),
            face_on_sha256=_digest(f"clip-{index if clip is None else clip}"),
            shot_sha256=(
                _digest(f"photo-{index}")
                if photo is True
                else None
                if photo is None
                else _digest(f"photo-{photo}")
            ),
            club=club,
            measurements=[
                Measurement(
                    name=name,
                    value=value,
                    unit=_CAREER_KINDS[name][0],
                    source=_CAREER_KINDS[name][1],
                    detail="synthetic",
                )
                for name, value in (values or {}).items()
            ],
            **fields,
        )

    def corpus(*swings: CorpusSwing, **scan: Any) -> CareerCorpus:
        """The swings in the reader's order, with the counts a reader would have filled."""
        scan.setdefault("sessions_scanned", len({s.session_id for s in swings}))
        scan.setdefault("swing_dirs_seen", len(swings) + sum(len(s.duplicates) for s in swings))
        return CareerCorpus(
            player_id=_REAL_PLAYER,
            swings=sorted(swings, key=lambda s: (s.captured_at, s.session_id, s.swing_id)),
            **scan,
        ).narrowed_to()

    def wave(center: float, spread: float, n: int, *, phase: float = 0.0) -> list[float]:
        """`n` values around `center`: irregular, deterministic and full-precision, so a port's
        sums and powers are exercised on more than the exact halves `tight` gives."""
        return [center + spread * math.sin(2.39996 * i + phase) for i in range(n)]

    def tight(center: float, half: float, n: int = 12) -> list[float]:
        """`tests/analysis/test_comparison.py::_tight`: a mean at `center`, a narrow interval."""
        return [center + (half if i % 2 == 0 else -half) for i in range(n)]

    def struck(carry: float, speed: float, sway: float) -> dict[str, float]:
        """A shot with a photo: the four launch-monitor readings a club profile pools, and a pose
        one beside them so a mishit can be seen counting there."""
        return {
            "carry_distance_yds": carry,
            "total_distance_yds": carry * 1.0625 + 0.3,
            "ball_speed_mph": speed,
            "launch_angle_deg": 31.0 - speed / 7.0,
            "head_sway_norm": sway,
        }

    cases: dict[str, dict[str, Any]] = {}

    def case(name: str, note: str, built: CareerCorpus, *, bag: Bag | None = None) -> None:
        assert name not in cases, f"two career cases are named {name}"
        cases[name] = _career_vector(
            f"synthetic/{name}",
            _career_given(built, bag=bag),
            note=note,
            source="CorpusSwing built by scripts/conformance_vectors.py, as tests/analysis/ does",
        )

    # ------------------------------------------------------------- the floors, at n - 1 and n

    floored = ("head_sway_norm", "tempo_ratio", "hip_shift_at_top_norm")
    floors = (
        (BaselineClaim.CENTER, "below", (4, 7, 5)),
        (BaselineClaim.CENTER, "at", (5, 8, 6)),
        (BaselineClaim.SPREAD, "below", (9, 14, 11)),
        (BaselineClaim.SPREAD, "at", (10, 15, 12)),
        (BaselineClaim.TREND, "below", (11, 17, 13)),
        (BaselineClaim.TREND, "at", (12, 18, 14)),
    )
    for claim, side, counts in floors:
        series = {
            "head_sway_norm": wave(0.21, 0.06, counts[0]),
            "tempo_ratio": wave(3.2, 0.45, counts[1], phase=1.0),
            "hip_shift_at_top_norm": wave(0.09, 0.035, counts[2], phase=2.0),
        }
        built = corpus(
            *(
                swing(
                    i + 1,
                    {name: values[i] for name, values in series.items() if i < len(values)},
                    session=weeks[i % 3],
                )
                for i in range(max(counts))
            )
        )
        metrics = build_baseline(built).metrics
        for name, n in zip(floored, counts, strict=True):
            assert metrics[name].n == n == minimum_n(name, claim) - (side == "below"), name
            assert (claim in metrics[name].ready) is (side == "at"), (claim, name, side)
        shy = "one sample short of" if side == "below" else "exactly at"
        case(
            f"floor-{claim.value}-{side}",
            f"head_sway_norm at {counts[0]}, tempo_ratio at {counts[1]} and hip_shift_at_top_norm "
            f"at {counts[2]} over three sessions: {shy} each one's {claim.value.upper()} floor "
            "(the default, the tempo override, the hip-shift override)",
            built,
        )

    for count in (1, 2, 3):
        built = corpus(
            *(
                swing(i + 1, {"head_sway_norm": value}, session=weeks[i % count])
                for i, value in enumerate(wave(0.24, 0.05, 12, phase=0.5))
            )
        )
        metric = build_baseline(built).metrics["head_sway_norm"]
        assert metric.n == 12 and metric.n_sessions == count
        assert (BaselineClaim.TREND in metric.ready) is (count == 3)
        case(
            f"trend-sessions-{count}",
            f"twelve samples, past every default floor, over {count} session(s): the TREND "
            f"sessions gate {'opens at 3' if count == 3 else 'refuses on sessions alone'}"
            + ("; one session has no within-session spread to pool" if count == 1 else ""),
            built,
        )

    thin = corpus(
        swing(
            1,
            {"head_sway_norm": 0.27, "face_to_path_deg": 10.9, "carry_distance_yds": 148.2},
            session=weeks[0],
            club=seven,
        ),
        swing(
            2,
            {"head_sway_norm": 0.31, "face_to_path_deg": 13.2, "carry_distance_yds": 151.7},
            session=weeks[0],
            club=seven,
        ),
    )
    assert build_baseline(thin).nothing_sayable
    case(
        "thin",
        "two swings in one session, the n on disk when career mode was built: every claim "
        "refused, and the TREND refusal names both shortfalls",
        thin,
    )

    # -------------------------------------------- pooling: mishits, flags and re-uploads

    pool = ("2026-08-20", "2026-08-21")
    carries = wave(152.0, 5.5, 7, phase=0.2)
    speeds = wave(118.0, 2.5, 7, phase=1.3)
    sways = wave(0.22, 0.05, 7, phase=2.1)
    unseen = "2026-08-22"
    pooled = corpus(
        *(
            swing(i + 1, struck(carries[i], speeds[i], sways[i]), session=pool[i % 2], club=seven)
            for i in range(7)
        ),
        swing(8, struck(21.3, 64.2, 0.31), session=pool[1], club=seven, auto_mishit=True),
        swing(
            9,
            struck(104.0, 97.5, 0.26),
            session=pool[0],
            club=seven,
            manual_mishit=MishitVerdict.CONFIRMED,
        ),
        swing(
            10,
            struck(66.5, 101.0, 0.19),
            session=pool[1],
            club=seven,
            auto_mishit=True,
            manual_mishit=MishitVerdict.CLEARED,
        ),
        swing(11, struck(149.0, 117.0, 0.24), session=pool[0], club=seven, shot_needs_review=True),
        swing(12, struck(150.4, 118.8, 0.21), session=pool[1], club=seven, photo=2),
        swing(13, struck(153.3, 119.2, 0.23), session=pool[0], club=seven, clip=4),
        swing(14, struck(151.1, 117.7, 0.2), session=pool[1], club=seven, photo=None),
        swing(15, {"head_sway_norm": 0.28, "carry_distance_yds": 233.0}, session=pool[0]),
        swing(16, struck(150.0, 118.0, 0.25), session=pool[1], club=seven, stale=True),
        swing(
            17,
            struck(150.0, 118.0, 0.25),
            session=pool[0],
            club=seven,
            outdated=True,
            analysis_version=ANALYSIS_VERSION - 1,
        ),
        swing(18, None, session=pool[1], club=seven),
        swing(
            19,
            struck(150.9, 118.4, 0.22),
            session=pool[0],
            club=seven,
            duplicates=[f"{unseen}/1", f"{unseen}/2"],
            conflicting_shots=[_digest("photo-19b"), _digest("photo-19c")],
        ),
        swing(
            20,
            {
                "mystery_reach_norm": 0.5,
                "pivot_hip_axis_drift_norm_dtl": 0.04,
                "tour_joint_distance": 1.3,
            },
            session=pool[1],
            club=seven,
        ),
        swing(21, None, session=pool[0], club=seven, analyzed=False),
        sessions_scanned=3,
        swing_dirs_seen=26,
        unattributed_swings=1,
        other_golfers=2,
        excluded=[
            ExcludedSwing(
                session_id=unseen,
                swing_id="3",
                reason=Reason.UNATTRIBUTED,
                detail="nobody had selected a golfer when this arrived — repair it on the upload "
                "page or with scripts/backfill_golfer.py",
            ),
            *(
                ExcludedSwing(
                    session_id=unseen,
                    swing_id=swing_id,
                    reason=Reason.DUPLICATE,
                    detail=f"face-on bytes {_digest('clip-19')[:12]} are already in "
                    f"{pool[0]}/19 — the same swing uploaded again, not a second swing",
                )
                for swing_id in ("1", "2")
            ),
            ExcludedSwing(
                session_id=pool[0],
                swing_id="21",
                reason=Reason.NOT_ANALYZED,
                detail="no analysis.json — run scripts/analyze_bundle.py over it",
            ),
            ExcludedSwing(
                session_id=pool[1],
                swing_id="16",
                reason=Reason.STALE,
                detail="a clip was re-uploaded after this was analyzed, so the stored numbers "
                "describe bytes that are no longer here — re-run the pipeline",
            ),
            ExcludedSwing(
                session_id=pool[0],
                swing_id="17",
                reason=Reason.OUTDATED,
                detail=f"analyzed by engine version {ANALYSIS_VERSION - 1}, and "
                f"{ANALYSIS_VERSION} is installed — the numbers are not comparable with a swing "
                "analyzed today, so they are reported rather than pooled. Re-run "
                "scripts/reanalyze.py",
            ),
        ],
    )
    pooled_baseline = build_baseline(pooled).metrics
    assert pooled.mishit_refs == [f"{pool[0]}/9", f"{pool[1]}/8"], pooled.mishit_refs
    assert pooled_baseline["carry_distance_yds"].n == 11, pooled_baseline["carry_distance_yds"].n
    assert pooled_baseline["ball_speed_mph"].n == 12, pooled_baseline["ball_speed_mph"].n
    assert pooled_baseline["head_sway_norm"].n == 15, pooled_baseline["head_sway_norm"].n
    for name, count in pooled.metric_counts.items():
        assert pooled_baseline[name].n == count, f"{name}: pooling disagrees with counting"
    case(
        "pooling",
        "one club's history with every pooling rule in it: an automatic and a confirmed mishit "
        "held out of carry and total and counted for ball speed and pose, a cleared one counted, "
        "a flagged parse, one photo under two clips, one clip under two swings, no photo, an "
        "untagged swing, stale, outdated and unanalyzed swings, re-uploads, a shot conflict, and "
        "unknown, down-the-line and population sources",
        pooled,
    )

    # ------------------------------------------------- dispersion: patterns, targets, drift

    def alternating(center: float, half: float, n: int = 12, *, jitter: float = 0.1) -> list[float]:
        """`tests/analysis/test_dispersion.py::_spread`, with a small irregular jitter on top."""
        return [
            center + (half if i % 2 == 0 else -half) + jitter * half * math.sin(1.7 * i + 0.4)
            for i in range(n)
        ]

    shapes: dict[str, tuple[list[float], Any]] = {
        "face_to_path_deg": (alternating(-9.0, 0.5), DispersionPattern.BIASED),
        "start_line_deg": (alternating(0.0, 10.0), DispersionPattern.SCATTERED),
        "start_line_offline_yds": (
            alternating(40.0, 25.0),
            DispersionPattern.BIASED_AND_SCATTERED,
        ),
        "head_sway_norm": (alternating(0.03, 0.04), DispersionPattern.NOTHING_ESTABLISHED),
        "finish_balance_norm": (alternating(0.2, 0.005), DispersionPattern.BIASED),
        "carry_distance_yds": (alternating(150.0, 30.0), SCATTER_ONLY_READING),
        "total_distance_yds": (alternating(160.0, 1.0), None),
        "flight_carry_yds": (alternating(151.0, 3.0), None),
        "pivot_hip_axis_drift_norm": (alternating(0.05, 0.01), None),
    }
    patterns = corpus(
        *(
            swing(
                i + 1,
                {name: values[i] for name, (values, _) in shapes.items()},
                session=weeks[i % 3],
            )
            for i in range(12)
        )
    )
    read = build_dispersion(patterns).metrics
    for name, (_, meant) in shapes.items():
        if isinstance(meant, DispersionPattern):
            assert read[name].pattern is meant, (name, read[name].pattern)
        elif meant == SCATTER_ONLY_READING:
            assert read[name].pattern is None and read[name].points_at == meant, name
            assert read[name].bias is Finding.WITHHELD, name
        else:
            assert read[name].pattern is None and read[name].points_at is None, name
    assert read["face_to_path_deg"].offset is not None and read["face_to_path_deg"].offset < 0
    assert any("not registered" in r for r in read["flight_carry_yds"].unavailable)
    case(
        "dispersion-patterns",
        "twelve swings over three sessions, one metric per reading: biased (a closed face), "
        "scattered, both, nothing established, a positive bias, scatter with no target (the "
        "scatter-only reading), a target-less metric inside its tolerance, and two metrics "
        "METRIC_TARGETS does not register (a model output and a pose metric)",
        patterns,
    )

    # Two tight sessions apart from each other. With six samples a session and a within-session sd
    # `w`, the pooled variance is (10 w^2 + 3 d^2) / 11 for a gap `d` between the session means, so
    # the pooled-over-within ratio crosses SESSION_DRIFT_FACTOR's 1.25 at d / w = 1.548. 1.6 lands
    # above it (1.268) and 1.5 below (1.234): one metric each side, in one corpus.
    offsets = (-1.2, -0.7, -0.1, 0.3, 0.6, 1.1)
    spread_of = math.sqrt(sum(o * o for o in offsets) / 5)

    def drifting(center: float, scale: float, gap: float) -> list[float]:
        step = gap * scale * spread_of
        return [center + scale * o for o in offsets] + [center + step + scale * o for o in offsets]

    drift = {
        "head_sway_norm": drifting(0.2, 0.02, 1.6),
        "hip_sway_norm": drifting(0.3, 0.03, 1.5),
    }
    drifted = corpus(
        *(
            swing(i + 1, {name: values[i] for name, values in drift.items()}, session=weeks[i // 6])
            for i in range(12)
        )
    )
    read = build_dispersion(drifted).metrics
    assert read["head_sway_norm"].caveats and read["head_sway_norm"].within_session_sd
    assert not read["hip_sway_norm"].caveats and read["hip_sway_norm"].within_session_sd
    case(
        "dispersion-drift",
        "two sessions of six, each tight and the two apart: head_sway_norm's pooled spread is "
        "1.268x its within-session spread (the drift caveat, past SESSION_DRIFT_FACTOR) and "
        "hip_sway_norm's 1.234x (no caveat)",
        drifted,
    )

    # ------------------------------------------------------- standing in the tour population

    def band(name: str) -> Any:
        found = load_distribution(name)
        assert found is not None, f"{name} has no stored distribution"
        return found

    sway, gain, hip, finish, tempo, down, back = (
        band(name)
        for name in (
            "head_sway_norm",
            "head_hip_gain_norm",
            "hip_sway_norm",
            "finish_balance_norm",
            "tempo_ratio",
            "downswing_ms",
            "backswing_ms",
        )
    )
    placed = {
        # 42.5th and 87.5th percentiles, which the baseline report prints `:.0f` — half to even,
        # so 42 and 88, where half-up would print 43 and 88.
        "head_sway_norm": tight(sway.p25 + 0.7 * (sway.p50 - sway.p25), 0.002),
        "head_hip_gain_norm": tight(gain.p75 + (5 / 6) * (gain.p90 - gain.p75), 0.002),
        "hip_sway_norm": tight(hip.p90 + 0.2, 0.002),
        "finish_balance_norm": tight(finish.p10 / 2, 0.001),
        "tempo_ratio": tight(tempo.p90, 0.3),
        "downswing_ms": tight(down.p50, 4.0),
        "backswing_ms": wave(back.p50, 60.0, 12, phase=0.7),
        "hip_shift_at_top_norm": wave(0.08, 0.02, 3),
        "head_hip_offset_impact_norm": tight(0.14, 0.01),
        "face_to_path_deg": tight(8.6, 0.5),
        "flight_carry_yds": wave(150.0, 4.0, 12, phase=1.1),
        "hand_height_norm": wave(0.6, 0.05, 12, phase=2.2),
        "flight_spin_rpm": wave(6100.0, 350.0, 12, phase=0.3),
    }
    standings = corpus(
        *(
            swing(
                i + 1,
                {name: values[i] for name, values in placed.items() if i < len(values)},
                session=weeks[i % 3],
            )
            for i in range(12)
        )
    )
    stood = build_standing(standings).metrics
    expect = {
        "head_sway_norm": Standing.INSIDE,
        "head_hip_gain_norm": Standing.INSIDE,
        "hip_sway_norm": Standing.OUTSIDE,
        "finish_balance_norm": Standing.OUTSIDE,
        "tempo_ratio": Standing.STRADDLES,
        "downswing_ms": Standing.INSIDE,
        "hip_shift_at_top_norm": Standing.WITHHELD,
        "head_hip_offset_impact_norm": Standing.WITHHELD,
        "face_to_path_deg": Standing.WITHHELD,
        "flight_carry_yds": Standing.WITHHELD,
        "hand_height_norm": Standing.WITHHELD,
    }
    for name, standing in expect.items():
        assert stood[name].standing is standing, (name, stood[name].standing)
    assert stood["head_sway_norm"].percentile == 42.5, stood["head_sway_norm"].percentile
    assert stood["head_hip_gain_norm"].percentile == 87.5, stood["head_hip_gain_norm"].percentile
    above, below = stood["hip_sway_norm"].outside_by, stood["finish_balance_norm"].outside_by
    assert above is not None and below is not None and above > 0 > below, (above, below)
    unplaced = stood["hip_shift_at_top_norm"]
    assert unplaced.band_low is not None and unplaced.withheld, "a band, and no center for it"
    case(
        "standings",
        "every Standing and every refusal beside one: inside (at the 42.5th and 87.5th "
        "percentiles, half-even under `:.0f`), above and below the band, straddling, a center "
        "the guard withheld, the blocked metric, and no population for a launch-monitor, a model "
        "and a pose metric; the spread line wherever SPREAD is ready",
        standings,
    )

    # ------------------------------------------------------------------- the bag profile

    def entry(club: ClubId, recorded: datetime, **spec: Any) -> BagEntry:
        return BagEntry(club=club, recorded_at=recorded, **spec)

    def bag(*entries: BagEntry, retired: tuple[BagEntry, ...] = ()) -> Bag:
        return Bag(
            player_id=_REAL_PLAYER,
            entries={one.club: one for one in entries},
            retired=retired,
            updated_at=on("2026-08-24"),
        )

    def hits(
        club: ClubId, start: int, carry: float, sessions: tuple[str, ...], **kw: Any
    ) -> list[CorpusSwing]:
        """One swing of `club` per session given, numbered from `start`, carries rising by a yard
        and a bit so no two are equal."""
        return [
            swing(
                start + i,
                struck(carry + 1.37 * i, carry / 1.32 + 0.4 * i, 0.2 + 0.011 * i),
                session=session,
                club=club,
                **kw,
            )
            for i, session in enumerate(sessions)
        ]

    no_bag = corpus(
        *hits(seven, 1, 151.0, weeks + weeks),
        *hits(driver, 11, 246.0, weeks[:2]),
        swing(21, {"head_sway_norm": 0.26}, session=weeks[0]),
        swing(22, {"head_sway_norm": 0.23}, session=weeks[2]),
    )
    case(
        "bag-none",
        "two clubs hit and two swings untagged, and no bag declared: every statistic stands, "
        "each club says what an undeclared entry costs, and the untagged swings are named last",
        no_bag,
    )

    hybrid = ClubId.FOUR_HYBRID
    declared_swings = corpus(
        *hits(seven, 1, 151.0, weeks + weeks[:2]),
        *hits(driver, 11, 246.0, weeks + weeks),
        *hits(ClubId.THREE_WOOD, 21, 221.0, weeks[1:2]),
        *hits(five, 31, 181.0, weeks),
        swing(
            41,
            struck(129.0, 99.0, 0.21),
            session=weeks[1],
            club=nine,
            at=datetime(2026, 8, 10, 12, 9, tzinfo=UTC),
        ),
        *hits(nine, 42, 128.0, weeks[2:]),
        *hits(hybrid, 51, 196.0, weeks[:2]),
        swing(61, {"head_sway_norm": 0.25}, session=weeks[2]),
    )
    early = on("2026-08-01")
    declared_bag = bag(
        entry(
            seven,
            early,
            make="Titleist",
            model="T250",
            loft_deg=30.5,
            shaft_model="KBS Tour",
            length_in=37.0,
            provenance=SpecProvenance(
                source="typed",
                retrieved_at=on("2026-08-01", 1),
                notes="published figures for the T250 7 iron, typed from the spec table",
            ),
        ),
        entry(driver, on("2026-08-09", hour=9)),
        entry(ClubId.THREE_WOOD, on("2026-08-20"), make="TaylorMade", loft_deg=15.0),
        # A late-evening entry five hours west of UTC: the caveat dates it in its own offset.
        entry(
            five,
            on("2026-08-20", 30, hour=23, tz=timezone(timedelta(hours=-5))),
            make="Mizuno Pro",
            model="245",
            loft_deg=24.0,
            shaft_model="Nippon Modus 105",
            length_in=38.25,
        ),
        # The same instant as the 9 iron's first swing, written in another offset: not earlier.
        entry(
            nine,
            on("2026-08-10", 39, hour=17, tz=timezone(timedelta(hours=5, minutes=30))),
            model="P790",
        ),
        entry(ClubId.PITCHING_WEDGE, early, loft_deg=46.0),
        retired=(
            entry(hybrid, early, make="Ping", model="G430", retired_at=on("2026-08-15")),
        ),
    )
    profile = build_bag_profile(declared_swings, declared_bag)
    caveats = {one.club: one.caveats for one in profile.clubs}
    assert caveats[seven] == [] and caveats[nine] == [] and caveats[hybrid] == []
    assert caveats[driver][0].startswith("2 of the 6 swings"), caveats[driver]
    assert caveats[ClubId.THREE_WOOD][0].startswith("The single swing"), caveats
    assert caveats[five][0].startswith("All 3 swings") and "2026-08-20" in caveats[five][0]
    assert not next(one for one in profile.clubs if one.club is hybrid).in_bag
    case(
        "bag-declared",
        "a declared bag against its history: an entry before every swing, one mid-history (the "
        "mixed caveat), one after a single swing and one after three (both all-predate forms, "
        "the second dated in its own -05:00 offset), one at the same instant as its first swing "
        "(strict <, no caveat), one never hit, and a retired club that keeps its history",
        declared_swings,
        bag=declared_bag,
    )

    confirmed = MishitVerdict.CONFIRMED
    flagged = {"club": seven, "auto_mishit": True}
    mishits = corpus(
        *hits(seven, 1, 151.0, weeks + weeks),
        swing(7, struck(97.0, 96.0, 0.24), session=weeks[0], club=seven, manual_mishit=confirmed),
        swing(8, struck(31.0, 70.0, 0.29), session=weeks[1], stale=True, **flagged),
        swing(9, struck(28.0, 66.0, 0.3), session=weeks[2], photo=None, **flagged),
        *hits(driver, 11, 246.0, weeks + weeks),
        swing(17, struck(41.0, 80.0, 0.31), session=weeks[0], club=driver, auto_mishit=True),
        swing(18, struck(55.0, 88.0, 0.27), session=weeks[2], club=driver, auto_mishit=True),
        *hits(five, 21, 181.0, weeks + weeks[:2]),
        swing(26, struck(62.0, 90.0, 0.22), session=weeks[1], club=five, auto_mishit=True),
        swing(27, struck(80.0, 95.0, 0.23), session=weeks[2], club=five, manual_mishit=confirmed),
        *hits(nine, 31, 128.0, weeks[:2]),
    )
    profile = build_bag_profile(mishits)
    counted = {one.club: (one.mishits, one.mishits_unconfirmed) for one in profile.clubs}
    assert counted == {driver: (2, 2), five: (2, 1), seven: (1, 0), nine: (0, 0)}, counted
    case(
        "bag-mishits",
        "the mishit caveat in each form: one confirmed (singular), two automatic (plural, both "
        "waiting), one of each (plural, one waiting); a stale and a photo-less flag that move no "
        "number and so are not counted; a clean club",
        mishits,
        bag=bag(entry(seven, early), entry(driver, early)),
    )

    case(
        "empty",
        "no swings and no bag: every aggregate empty and every report's empty state",
        corpus(),
    )
    case(
        "untagged-only",
        "three swings naming no club and no bag: no profile at all, and the empty bag report "
        "names the untagged history instead",
        corpus(
            *(swing(i + 1, {"head_sway_norm": 0.2 + 0.03 * i}, session=weeks[i]) for i in range(3))
        ),
    )
    case(
        "declared-unhit",
        "a declared bag and no swings: two profiles with nothing behind them",
        corpus(),
        bag=bag(
            entry(ClubId.SAND_WEDGE, early, make="Vokey", model="SM10", loft_deg=56.0),
            entry(ClubId.LOB_WEDGE, early, make="Vokey", loft_deg=60.0, length_in=35.125),
        ),
    )
    case(
        "unmeasured-club",
        "one club whose swings carry nothing: two analyzed with no measurement and no photo, one "
        "never analyzed, so the profile names the shot ceiling and why it has no history",
        corpus(
            swing(1, None, session=weeks[0], club=seven, photo=None),
            swing(2, None, session=weeks[1], club=seven, photo=None),
            swing(3, None, session=weeks[2], club=seven, photo=None, analyzed=False),
        ),
    )

    # ---------------------------------------------- the statistics past their tables, and edges

    # `analysis/stats.py` tabulates both critical values for df 1..30 and expands beyond them
    # (Cornish-Fisher for t, Wilson-Hilferty for chi-square), so n = 31 is the tables' last row,
    # 32 the expansions' first and 75 deep inside them: the float powers a port has to match.
    lone = "2026-08-24"
    sway_band = band("head_sway_norm")
    edges = {
        "head_sway_norm": wave(sway_band.p10 + 0.5 * (sway_band.p25 - sway_band.p10), 0.004, 31),
        "hip_sway_norm": wave(0.3, 0.05, 32, phase=0.9),
        "finish_balance_norm": wave(0.15, 0.03, 75, phase=1.7),
        "launch_angle_deg": [12.5] * 12,
        "face_to_path_deg": tight(-0.02, 0.5),
    }
    past = corpus(
        *(
            swing(
                i + 1,
                {name: values[i] for name, values in edges.items() if i < len(values)},
                session=lone if i == 74 else weeks[i % 3],
            )
            for i in range(75)
        )
    )
    metrics = build_baseline(past).metrics
    assert [metrics[name].n for name in edges] == [31, 32, 75, 12, 12]
    assert metrics["finish_balance_norm"].n_sessions == 4, "the lone session is the fourth"
    assert metrics["launch_angle_deg"].sd == 0.0
    placed_at = build_standing(past).metrics["head_sway_norm"].percentile
    assert placed_at is not None and 10.0 < placed_at < 25.0, placed_at
    case(
        "past-the-tables",
        "n = 31, 32 and 75: the critical-value tables' last row, the expansions' first and one "
        "deep inside them; a fourth session holding one sample, which the within-session spread "
        "leaves out; twelve identical values (sd 0); a center that prints -0.0; and a center "
        "between the band's p10 and p25",
        past,
    )

    # Two sessions whose first samples are one instant written in two offsets. `_sessions_of`
    # sorts on (captured_at, session_id), so the tie falls to the id, and "2026-08-03" leads even
    # though its lexeme, 14:01+02:00, sorts after the other's 12:01Z.
    tie = ("2026-08-03", "2026-08-03-bay2", "2026-08-04")
    days = ("2026-08-03", "2026-08-03", "2026-08-04")
    firsts = (on(days[0], 1, hour=14, tz=timezone(timedelta(hours=2))), on(days[1], 1))
    tied = corpus(
        *(
            swing(
                i + 1,
                {"head_sway_norm": value},
                session=tie[i % 3],
                at=firsts[i] if i < 2 else on(days[i % 3], i + 1),
            )
            for i, value in enumerate(wave(0.19, 0.04, 12, phase=0.4))
        )
    )
    sessions = build_baseline(tied).metrics["head_sway_norm"].sessions
    assert [session.session_id for session in sessions] == list(tie), sessions
    case(
        "session-tie",
        "two sessions whose first samples are the same instant in two offsets: the per-session "
        "order falls to the session id, not to the timestamps' text",
        tied,
    )
    return cases


# -------------------------------------------------------------- career: adopted, and the real one


def _career_adopted() -> dict[str, dict[str, Any]]:
    """Every synthetic storage corpus case's corpus, as the reader answered it, rendered.

    Read from the committed family rather than rebuilt, so each corpus is the one its storage
    vector certifies, and checked to come back unchanged through `model_validate` and `model_dump`.
    A golfer other than the real one is named as the scripts name an unregistered id.
    """
    from golf_coach.contracts.career import CareerCorpus

    directory = VECTORS / "storage" / "corpus"
    paths = sorted(directory.glob("*.json"))
    if not paths:
        raise AssertionError(
            f"nothing under {_rel(directory)} to adopt — `regenerate --storage-once` records it"
        )
    cases: dict[str, dict[str, Any]] = {}
    for path in paths:
        stored = json.loads(path.read_text(encoding="utf-8"))
        dumped = stored["expected"]["corpus"]
        corpus = CareerCorpus.model_validate(dumped)
        assert corpus.model_dump(mode="json") == dumped, f"{path.name} does not round-trip"
        name = f"storage-{path.stem}"
        display = (
            "Aaron" if corpus.player_id == _REAL_PLAYER else f"{corpus.player_id} (not registered)"
        )
        given = _career_given(corpus, display_name=display)
        given["corpus"] = dumped
        cases[name] = _career_vector(
            f"synthetic/{name}",
            given,
            note=f"the corpus storage/corpus/{path.stem} reads, adopted whole and rendered",
            source=f"{_rel(path)}, expected.corpus",
        )
    return cases


def _career_real() -> dict[str, Any]:
    """The golfer on disk: the real storage vector's corpus, their bag and their name.

    The corpus is the one `spec/vectors/storage/corpus/real.json.gz` records, and it must equal
    `read_corpus` over `data/` itself or this raises (the plan's call 2), so the two families start
    from one corpus. The mishit listing is then run over the real trees as well, the storage
    vector's sessions materialised beside a copy of the golfers directory, and must print what the
    case recorded.
    """
    import gzip
    import shutil
    from types import SimpleNamespace

    from golf_coach.contracts.career import CareerCorpus
    from golf_coach.storage.bag_store import BagStore
    from golf_coach.storage.corpus import read_corpus
    from golf_coach.storage.golfer_store import GolferStore

    for needed in (SESSIONS, GOLFERS):
        if not needed.is_dir():
            raise AssertionError(f"{_rel(needed)} is missing — the real career case reads it")
    stored_path = VECTORS / "storage" / "corpus" / f"real{_REAL_SUFFIX}"
    stored = json.loads(gzip.decompress(stored_path.read_bytes()))
    dumped = stored["expected"]["corpus"]
    if read_corpus(SESSIONS, _REAL_PLAYER).model_dump(mode="json") != dumped:
        raise AssertionError(
            f"{_rel(SESSIONS)} no longer reads as {_rel(stored_path)} recorded it, so the career "
            "family would start from a different corpus than the storage family"
        )
    corpus = CareerCorpus.model_validate(dumped)
    assert corpus.model_dump(mode="json") == dumped, "the real corpus does not round-trip"

    with tempfile.TemporaryDirectory() as scratch:
        golfers = Path(scratch) / "golfers"
        shutil.copytree(GOLFERS, golfers)
        golfer = GolferStore(golfers).get(_REAL_PLAYER)
        assert golfer is not None, f"no golfer record for {_REAL_PLAYER}"
        given = _career_given(
            corpus, bag=BagStore(golfers).get(_REAL_PLAYER), display_name=golfer.display_name
        )
        given["corpus"] = dumped
        vector = _career_vector(
            f"real/{_REAL_PLAYER}",
            given,
            note=(
                "the golfer on disk: the real storage vector's corpus, verified to equal "
                "read_corpus over data/, with the declared bag and the registered name"
            ),
            source=f"{_rel(stored_path)} expected.corpus, and {_rel(GOLFERS)}",
        )

        sessions = Path(scratch) / "sessions"
        _materialise(stored["input"]["files"], sessions)
        module = _career_scripts()["flag_mishit"]
        recorded = vector["expected"]["reports"]["flag_mishit_list"]
        settings = SimpleNamespace(sessions_dir=sessions, golfers_dir=golfers)
        with _patched(module, settings=settings):
            for only in (None, _REAL_PLAYER):
                over_trees = _printed(module._list, None, GolferStore(golfers), only)
                if over_trees != recorded:
                    raise AssertionError(
                        f"`flag_mishit --list` over the real trees (only={only}) prints "
                        f"{over_trees!r}, and over the corpus {recorded!r}"
                    )
    return vector
