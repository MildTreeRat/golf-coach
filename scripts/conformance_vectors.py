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
format tables (M22 P3, M34 P2), and the screen family (M34 P4), which is recorded once, by
`regenerate --screen-once`, and never rebuilt from here.
"""

from __future__ import annotations

import copy
import importlib.util
import json
import math
import random
import re
import struct
import sys
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

    The last five tables are the screen parser's (M34 P2), and see their own section below.
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
