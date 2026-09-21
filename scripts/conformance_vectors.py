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
