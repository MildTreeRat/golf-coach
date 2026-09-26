"""Replay the stored clips through the live trigger and score it. [M20 P3]

    python scripts/trigger_replay.py                    # every stored clip, at the shipped bar
    python scripts/trigger_replay.py --sweep            # precision and recall across thresholds
    python scripts/trigger_replay.py --concat           # clips joined by silence, as one stream
    python scripts/trigger_replay.py --id 2026-08-23/2

**What the ground truth is, and what it is not.** For each clip it is the strike
`analysis/alignment.py::with_measured_impact` *chose* — one per view, read out of the stored
`analysis.json` and matched back to `{role}.audio.json` by frame (`_anchored` below). It is **not**
`detect_strikes`' output, and the first version of this script made that mistake: the offline
detector lists every transient a shot makes and some no shot made, so clustering its output scored
a golfer setting up at 0.37 s as a swing the live detector had missed.

**What it cannot measure.** A false positive in an empty bay, because no such recording exists —
every clip on disk was recorded around a swing. `--concat` stands in for part of it by joining the
clips with silence between, which catches a detector that fires on a seam or on its own warm-up,
and that is not the same thing. The real number needs the continuous recording M20 P4 asks for;
`docs/BAY_SESSION_RUNBOOK.md` is where that trip is sequenced.

The ground truth is also **incomplete, and knowing that changed the answer**:
`2026-08-23/8 down_the_line` is 80 seconds long and holds a second shot the corpus never labelled,
so the detector is scored as firing spuriously on a real swing. See `stream::MIN_TRIGGER_Z`.

**Why Python is still here.** This is lab work — tier 4 in `docs/CONFORMANCE.md` §5, alongside the
corpus tools and the model fitting — and it is the half that has to decode. `audio/ffmpeg.py`
holds the container subtleties (edit lists, the two `soun` tracks on the face-on clips) and
reaches ffmpeg through the `imageio-ffmpeg` wheel rather than a system install. Decoding is the
next thing to move to Rust; detection already has.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parent.parent
SESSIONS = REPO / "data" / "processed" / "sessions"
_NAME = "golf-trigger.exe" if sys.platform == "win32" else "golf-trigger"
BINARY = REPO / "target" / "release" / _NAME

#: The thresholds `--sweep` walks. Bracketing `stream::MIN_TRIGGER_Z`, which is provisional until
#: this script has been run: it was chosen from the offline measurement (room tone never above
#: z = 4.4, real strikes at z = 113-599) and those are whole-clip numbers, where the live floor is
#: a rolling five-second one.
_SWEEP = (8.0, 12.0, 20.0, 30.0, 45.0, 70.0, 110.0, 170.0, 260.0, 400.0)

#: A gap of pure silence between concatenated clips. Long enough that neither clip's tail can
#: reach the next one's head through the five-second floor window, so a trigger on the seam is a
#: real finding rather than two clips bleeding into each other.
_SILENCE_S = 8.0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--sweep", action="store_true", help="precision/recall across thresholds")
    parser.add_argument("--concat", action="store_true", help="join the clips into one stream")
    parser.add_argument("--id", action="append", help="only these swings, e.g. 2026-08-23/2")
    parser.add_argument("--min-z", type=float, help="a single threshold to run at")
    args = parser.parse_args(argv)

    if not BINARY.exists():
        print(f"{BINARY} is missing — run `cargo build --release`", file=sys.stderr)
        return 2
    try:
        clips = list(_decode(args.id))
    except ImportError:
        print("the audio extra is not installed — pip install -e '.[audio]'", file=sys.stderr)
        return 2
    if not clips:
        print(f"no clips under {SESSIONS}", file=sys.stderr)
        return 2

    if args.concat:
        return _report_concat(clips, args.min_z)
    if args.sweep:
        return _report_sweep(clips)
    return _report(clips, args.min_z)


def _decode(only: list[str] | None) -> Any:
    """Every stored clip as (name, rate, PCM bytes, ground-truth shot samples)."""
    from golf_coach.audio.ffmpeg import FfmpegAudioSource
    from golf_coach.storage.manifest import SwingManifest

    wanted = set(only or [])
    for manifest_path in sorted(SESSIONS.glob("*/*/manifest.json")):
        swing_dir = manifest_path.parent
        key = f"{swing_dir.parent.name}/{swing_dir.name}"
        if wanted and key not in wanted:
            continue
        manifest = SwingManifest.model_validate_json(manifest_path.read_text(encoding="utf-8"))
        for role, role_file in sorted(manifest.roles.items(), key=lambda kv: kv[0].value):
            path = swing_dir / role_file.filename
            if path.suffix.lower() not in {".mov", ".mp4", ".m4v"}:
                continue
            clip = FfmpegAudioSource(path, camera_id=role.value).read()
            yield (
                f"{key} {role.value}",
                clip.sample_rate,
                clip.samples.tobytes(),
                _anchored(swing_dir, role.value),
            )


def _anchored(swing_dir: Path, role: str) -> list[int]:
    """Where the pipeline decided the ball was, as sample indices in this clip.

    **The honest ground truth, and it is not `detect_strikes`' output.** The offline detector
    lists every transient a shot makes and some that no shot made — on
    `2026-08-07-aaron1/1 down_the_line` it lists one at 0.37 s with a third of the real strike's
    prominence, which is the golfer setting up. Scoring the live trigger against that list counts
    a setup noise as a swing it missed.

    What is unambiguous is the strike `analysis/alignment.py::with_measured_impact` *chose* —
    nearest to the pose impact, recorded in `analysis.json` as `alignment.{a,b}.anchors.impact`
    with `impact_measured` true. Matching it back to the stored `{role}.audio.json` by frame gives
    the sample exactly, with no fps arithmetic to get wrong (`_frames_derived` floors, so
    inverting it would not round-trip).

    Returns an empty list when this view has no acoustically measured impact, which is a clip that
    cannot contribute to recall rather than one the detector failed on.
    """
    analysis_path = swing_dir / "analysis.json"
    audio_path = swing_dir / f"{role}.audio.json"
    if not analysis_path.exists() or not audio_path.exists():
        return []
    alignment = (json.loads(analysis_path.read_text(encoding="utf-8")).get("alignment") or {})
    frames = {
        anchors["impact"]
        for side in ("a", "b")
        if (anchors := (alignment.get(side) or {}).get("anchors"))
        and anchors.get("camera_id") == role
        and anchors.get("impact_measured")
        and anchors.get("impact") is not None
    }
    strikes = json.loads(audio_path.read_text(encoding="utf-8")).get("strikes", [])
    return sorted(s["sample"] for s in strikes if s.get("frame") in frames)


def _run(
    pcm: bytes, rate: int, min_z: float | None, shots: list[int], command: str = "replay"
) -> dict[str, Any]:
    argv = [str(BINARY), command, "--rate", str(rate)]
    if command == "replay":
        argv += ["--shots", ",".join(str(s) for s in shots)]
    if min_z is not None:
        argv += ["--min-z", str(min_z)]
    done = subprocess.run(argv, input=pcm, capture_output=True, check=True)
    return json.loads(done.stdout)


def _report(clips: list[tuple[str, int, bytes, list[int]]], min_z: float | None) -> int:
    print(f"{'clip':<34} {'shots':>5} {'fired':>5} {'found':>5} {'miss':>4} {'extra':>5}  latency")
    totals = {"shots": 0, "triggers": 0, "found": 0, "missed": 0, "spurious": 0}
    latencies: list[float] = []
    for name, rate, pcm, shots in clips:
        r = _run(pcm, rate, min_z, shots)
        for key in totals:
            totals[key] += r[key]
        late = [s / rate * 1000 for s in r["latency_samples"]]
        latencies += late
        shown = ", ".join(f"{value:+.0f}ms" for value in late) or "-"
        print(
            f"{name:<34} {r['shots']:>5} {r['triggers']:>5} {r['found']:>5} "
            f"{r['missed']:>4} {r['spurious']:>5}  {shown}"
        )
    _summary(totals, latencies)
    return 0


def _summary(totals: dict[str, int], latencies: list[float]) -> None:
    recall = totals["found"] / totals["shots"] if totals["shots"] else 0.0
    precision = totals["found"] / totals["triggers"] if totals["triggers"] else 0.0
    print(
        f"\n{totals['found']}/{totals['shots']} shots found (recall {recall:.3f}), "
        f"{totals['spurious']} spurious of {totals['triggers']} triggers "
        f"(precision {precision:.3f})"
    )
    if latencies:
        ordered = sorted(latencies)
        print(
            f"latency: median {ordered[len(ordered) // 2]:+.0f} ms, "
            f"range {ordered[0]:+.0f} to {ordered[-1]:+.0f} ms"
        )


def _report_sweep(clips: list[tuple[str, int, bytes, list[int]]]) -> int:
    print(f"{'min_z':>7} {'recall':>7} {'prec':>7} {'found':>6} {'missed':>7} {'spurious':>9}")
    for z in _SWEEP:
        totals = {"shots": 0, "triggers": 0, "found": 0, "missed": 0, "spurious": 0}
        for _, rate, pcm, shots in clips:
            r = _run(pcm, rate, z, shots)
            for key in totals:
                totals[key] += r[key]
        recall = totals["found"] / totals["shots"] if totals["shots"] else 0.0
        precision = totals["found"] / totals["triggers"] if totals["triggers"] else 0.0
        print(
            f"{z:>7.0f} {recall:>7.3f} {precision:>7.3f} {totals['found']:>6} "
            f"{totals['missed']:>7} {totals['spurious']:>9}"
        )
    return 0


def _report_concat(clips: list[tuple[str, int, bytes, list[int]]], min_z: float | None) -> int:
    """Every clip as one continuous stream, with silence between.

    The nearest thing on disk to a continuous bay recording. It catches two failures a per-clip
    run cannot: a trigger on the join between two clips, and a detector whose rolling floor never
    recovers after a loud passage. It does *not* substitute for the real recording M20 P4 wants —
    silence is not room tone, and a false positive on a voice is exactly what this cannot show.
    """
    rate = clips[0][1]
    mismatched = [name for name, r, _, _ in clips if r != rate]
    if mismatched:
        print(f"clips at more than one sample rate: {mismatched}", file=sys.stderr)
        return 2
    gap = b"\x00\x00" * int(_SILENCE_S * rate)
    stream = gap.join(pcm for _, _, pcm, _ in clips)
    # Ground truth moves with the join: each clip's shots shift by every clip and gap ahead of it.
    shots: list[int] = []
    offset = 0
    for index, (_, _, pcm, clip_shots) in enumerate(clips):
        if index:
            offset += len(gap) // 2
        shots += [offset + s for s in clip_shots]
        offset += len(pcm) // 2
    r = _run(stream, rate, min_z, shots)
    seconds = len(stream) // 2 / rate
    print(
        f"{len(clips)} clips joined by {_SILENCE_S:.0f} s of silence: "
        f"{seconds:.0f} s, {r['shots']} shots, {r['triggers']} triggers"
    )
    _summary(
        {k: r[k] for k in ("shots", "triggers", "found", "missed", "spurious")},
        [s / rate * 1000 for s in r["latency_samples"]],
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
