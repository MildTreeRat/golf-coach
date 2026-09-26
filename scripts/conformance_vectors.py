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
"""

from __future__ import annotations

import importlib.util
import json
import math
import random
import sys
from pathlib import Path
from typing import Any

from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints, KeypointsFile
from golf_coach.contracts.swing import ANALYSIS_VERSION

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
    path = REPO / "tests" / "analysis" / "conftest.py"
    spec = importlib.util.spec_from_file_location("_conformance_conftest", path)
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
# under test is `crates/analysis/src/pyfmt.rs`, so `conformance.py check` defers these the way it
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

    `stage_vectors` is where `_engine_floats` gets its realistic inputs; `None` reads them from
    disk. **Unlike the stages family, a stale read here is harmless**, and the asymmetry is worth
    naming because the two functions look alike. A stage vector records an *answer* the engine
    gave, so building one against a superseded input produces a file that is self-consistent and
    wrong. This family takes only *inputs* from over there — every `expected` comes from CPython's
    `round` and `format`, which do not know what an engine is — so a value that has since stopped
    being produced is still a float Python rounds exactly the same way.

    Uncompressed, unlike the other families of comparable size: these files are meant to be *read*
    when a Rust test fails, and a reader who has to decompress a vector to see that 0.145 rounds to
    0.14 will guess instead.
    """
    pool = _engine_floats(stage_vectors)
    return [
        (VECTORS / "format" / "rounding.json", _format_rounding(pool)),
        (VECTORS / "format" / "fixed.json", _format_fixed(pool)),
        (VECTORS / "format" / "general.json", _format_general(pool)),
        (VECTORS / "format" / "repr.json", _format_repr(pool)),
        (VECTORS / "format" / "ordering.json", _format_ordering()),
    ]


def _format_vector(vector_id: str, note: str, cases: list[dict[str, Any]]) -> dict[str, Any]:
    assert cases, f"{vector_id} recorded no cases"
    return {
        "id": vector_id,
        # Deliberately not `analysis_version` — see this section's header. `check` keys its
        # staleness test on the field *name*, so carrying the engine's would make every version
        # bump report four stale vectors with nothing to regenerate about them.
        "python_version": ".".join(str(part) for part in sys.version_info[:3]),
        "provenance": {
            "kind": "format",
            "note": note,
            "edges": "ADR-032 §3; docs/CONFORMANCE.md §3 'The known edges a Rust port will hit'",
        },
        "cases": cases,
    }


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


def _engine_floats(
    stage_vectors: list[dict[str, Any]] | None = None, limit: int = 120
) -> list[float]:
    """Unrounded values the engine really hands to `round`, read off the stage vectors.

    `spec/vectors/stages/*`'s `measure` stage records each `POSE_MEASUREMENTS` entry's value
    *before* `_measurements` rounds it, which makes it the only place on disk holding this edge's
    real inputs. Reading them costs nothing and needs no captures — the same property the stages
    family itself has (ADR-032 §2).
    """
    if stage_vectors is None:
        from conformance import _read_json, stage_vector_paths

        stage_vectors = [_read_json(path) for path in stage_vector_paths()]

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
