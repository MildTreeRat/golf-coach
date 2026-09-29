"""Replay the stored clips through the pose sidecar and diff each result against disk. [M23 P7]

    python scripts/pose_replay.py                       # all 30 clips, ~77 minutes
    python scripts/pose_replay.py --id 2026-08-23/11    # one swing, both views
    python scripts/pose_replay.py --role face_on        # one view of every swing
    python scripts/pose_replay.py --out DIR             # keep the written files in DIR
    python scripts/pose_replay.py --shortest 6 --sweep 1,2,4   # M23 P8: how wide a pool pays

**Two modes, and the second one diffs nothing.** Without `--sweep` this is P7's harness: one
`golf-pose run` per clip, each result compared against the file on disk. With `--sweep` it is P8's:
the same clips through `golf-pose sweep` at each worker count in turn, timed and sampled, to answer
what a second and a fourth warm worker actually buy. The sweep does not compare against disk —
`golf-pose sweep` writes nothing, because P7 already established the landmarks are right and a
measurement of CPU should not have 100 MB of JSON writing inside it. What it does compare is the
configurations against *each other*: the binary prints a digest per reply, and a digest that moved
when the pool got wider would mean contention changed an answer.

**What the ground truth is.** The 30 `{role}.keypoints.json` files under
`data/processed/sessions/`, written by `api/pipeline.py` calling `estimate_pose` in-process. They
are the *only* oracle M23 has: ADR-033's "Consequences" declined a vector family for pose because
its true input is a 4K `.MOV` that cannot be committed and a keypoints-only family would be 239 MB
before gzip, so what stands in for it is `golf-pose` plus this diff. The plan
`docs/plans/m23-pose-sidecar.md` orders this run **before** §M30's clip trimming for that reason:
trimming re-cuts the clips, which changes their frame numbering and their sha256, and invalidates
all 30 at once.

**Why it reports a census and not a verdict.** M22 P8's finding — a tolerance far above a ulp
cannot tell a converged computation from a systematically wrong one — applies in reverse here:
"0 differing of 5,756,442" is a much stronger claim than "passed", and the two cost the same to
print. So every run states clips, frames, values, values differing, the worst absolute delta and
the wall clock, whichever way it came out.

**Structural, not byte-for-byte**, per ADR-033 clause 8: every parsed value exactly equal, and the
set of *key paths* equal on both sides so that an absent key cannot pass for a null. The one thing
deliberately not compared is key **order** — `serde_json::Map` is a `BTreeMap` and pydantic writes
in field-declaration order, which P6 measured as the last and only difference between the two
writers. Nothing hashes these files (the pose cache keys on the *clip's* sha256), so order is not a
property this repo has.

**Why Python is still here.** Lab work — tier 4 in `docs/CONFORMANCE.md` §5, beside
`scripts/trigger_replay.py`, which is this script's precedent down to the `--id` flag. The
comparison half has to read the committed files, and `storage/keypoints_io.load_keypoints` is the
tolerant reader for those; a second one here is the drift `CLAUDE.md` names by example.

**Why the sweep samples from out here rather than from inside the pool.** Aggregate CPU and peak
resident memory are facts about a *process tree* — one `golf-pose`, N CPython workers — and the
parent is the wrong place to ask: a worker's memory is not in the parent's address space and its CPU
time is only readable while it lives. `psutil` reads both from outside, and it arrives with the
`vision` extra (`ultralytics` requires it), which is the same extra the worker needs to pose at all,
so a box that can run this sweep can sample it. Without it the sweep still reports wall clock and
says which numbers it could not take, rather than reporting a guess.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from collections.abc import Iterator
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parent.parent
SESSIONS = REPO / "data" / "processed" / "sessions"
_NAME = "golf-pose.exe" if sys.platform == "win32" else "golf-pose"
BINARY = REPO / "target" / "release" / _NAME

#: Containers this script will ask the sidecar to decode. The same set `trigger_replay.py` walks,
#: minus nothing: every stored view is a QuickTime file today and `shot_screen` is an image.
_VIDEO = {".mov", ".mp4", ".m4v"}


class Clip:
    """One stored view: the container to re-pose, and the answer it has to reproduce."""

    def __init__(self, key: str, role: str, video: Path, committed: Path) -> None:
        self.key = key
        self.role = role
        self.video = video
        self.committed = committed

    @property
    def name(self) -> str:
        return f"{self.key} {self.role}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--id", action="append", help="only these swings, e.g. 2026-08-23/11")
    parser.add_argument("--role", action="append", help="only these views, e.g. face_on")
    parser.add_argument("--out", type=Path, help="write the keypoints here and keep them")
    parser.add_argument("--python", help="the interpreter golf-pose should run workers with")
    parser.add_argument("--variant", help="pose model variant; default is the worker's")
    parser.add_argument(
        "--shortest", type=int, metavar="N", help="only the N shortest of the selected clips"
    )
    parser.add_argument(
        "--sweep",
        metavar="N,N,...",
        help="measure pool sizes instead of diffing, e.g. 1,2,4",
    )
    args = parser.parse_args(argv)

    if not BINARY.exists():
        print(f"{BINARY} is missing — run `cargo build --release`", file=sys.stderr)
        return 2
    clips = list(_clips(args.id, args.role))
    if not clips:
        print(f"no clips with committed keypoints under {SESSIONS}", file=sys.stderr)
        return 2
    if args.shortest:
        clips = _shortest(clips, args.shortest)

    if args.sweep:
        try:
            sizes = [int(size) for size in args.sweep.split(",") if size.strip()]
        except ValueError:
            print(f"--sweep wants worker counts, not {args.sweep!r}", file=sys.stderr)
            return 2
        if not sizes or min(sizes) < 1:
            print("--sweep wants at least one worker count of 1 or more", file=sys.stderr)
            return 2
        return _sweep(clips, sizes, args.python, args.variant)

    # A temporary directory unless one was asked for, because a keypoints file written into a swing
    # directory would overwrite the oracle. `golf-pose` already refuses that without `--force`; this
    # is the same guard one level up, where the path is actually chosen.
    keep = args.out is not None
    out_root = args.out or Path(tempfile.mkdtemp(prefix="pose-replay-"))
    try:
        return _report(clips, out_root, args.python, args.variant)
    finally:
        if not keep:
            shutil.rmtree(out_root, ignore_errors=True)


def _clips(only: list[str] | None, roles: list[str] | None) -> Iterator[Clip]:
    """Every stored view that has both a container and a committed keypoints file.

    Driven off `manifest.json` rather than off globbing the `.MOV` files, because the manifest is
    what maps a role to a filename — the sha fragment in `face_on.21831919bc67.MOV` is
    `crates/capture`'s doing and is not derivable from the role.
    """
    from golf_coach.storage.manifest import SwingManifest

    wanted = set(only or [])
    wanted_roles = set(roles or [])
    for manifest_path in sorted(SESSIONS.glob("*/*/manifest.json")):
        swing_dir = manifest_path.parent
        key = f"{swing_dir.parent.name}/{swing_dir.name}"
        if wanted and key not in wanted:
            continue
        manifest = SwingManifest.model_validate_json(manifest_path.read_text(encoding="utf-8"))
        for role, role_file in sorted(manifest.roles.items(), key=lambda kv: kv[0].value):
            if wanted_roles and role.value not in wanted_roles:
                continue
            video = swing_dir / role_file.filename
            if video.suffix.lower() not in _VIDEO:
                continue
            committed = swing_dir / f"{role.value}.keypoints.json"
            if not committed.exists():
                continue
            yield Clip(key, role.value, video, committed)


def _pose(clip: Clip, out_root: Path, python: str | None, variant: str | None) -> tuple[Path, str]:
    """Run one clip through `golf-pose` and return where it landed.

    `--camera-id` is passed explicitly and that is load-bearing: the binary's fallback names the
    *file* from the clip's leading dot-component but leaves `FrameKeypoints.camera_id` unset, so a
    replay that omitted it would differ from the committed file on every frame.
    """
    out_dir = out_root / clip.key
    argv = [
        str(BINARY),
        "run",
        str(clip.video),
        "--out",
        str(out_dir),
        "--camera-id",
        clip.role,
        "--force",
    ]
    if python:
        argv += ["--python", python]
    if variant:
        argv += ["--variant", variant]
    done = subprocess.run(argv, capture_output=True, text=True)
    if done.returncode != 0:
        detail = (done.stderr or done.stdout).strip().splitlines()
        raise RuntimeError(detail[-1] if detail else f"exit {done.returncode}")
    return out_dir / f"{clip.role}.keypoints.json", done.stdout


def _pairs(ours: Any, theirs: Any, frames: int) -> Iterator[tuple[str, Any, Any]]:
    """Every scalar in the two files side by side, as (path, ours, theirs), envelope included.

    Both sides are walked *together* rather than zipped from two independent walks, so that the
    census can never be shortened by the very thing it is looking for: an absent field, or a frame
    with fewer landmarks than its counterpart, is yielded as `None` against a value and counts as a
    difference. `frames` bounds it at the shorter of the two files — a frame-count mismatch is
    reported on its own line, and walking past it would report the same fact a few hundred thousand
    times.
    """
    for field in ("fps", "width", "height", "frame_count", "source_sha256"):
        yield f"clip.{field}", _field(ours.clip, field), _field(theirs.clip, field)
    yield "pose_estimator", ours.pose_estimator, theirs.pose_estimator
    for index in range(frames):
        mine, yours = ours.frames[index], theirs.frames[index]
        for field in ("frame_index", "timestamp_ms", "camera_id"):
            yield f"frames[{index}].{field}", getattr(mine, field), getattr(yours, field)
        for number in range(max(len(mine.landmarks), len(yours.landmarks))):
            a = mine.landmarks[number] if number < len(mine.landmarks) else None
            b = yours.landmarks[number] if number < len(yours.landmarks) else None
            for field in ("x", "y", "z", "visibility"):
                path = f"frames[{index}].landmarks[{number}].{field}"
                yield path, _field(a, field), _field(b, field)


def _field(owner: Any, field: str) -> Any:
    """One field of an optional sub-object: `None` when the object itself is absent."""
    return None if owner is None else getattr(owner, field)


def _key_paths(payload: Any, prefix: str = "") -> set[str]:
    """The set of key paths in the raw JSON, with array indices collapsed to `[]`.

    This is the half `load_keypoints` cannot answer: an absent key and an explicit `null` both parse
    to `None`, and the writer's whole job is to keep them apart (`exclude_none=True` on one side, a
    `Value` walk on the other). Sixteen paths for a full file.
    """
    if isinstance(payload, dict):
        found: set[str] = set()
        for key, value in payload.items():
            path = f"{prefix}.{key}" if prefix else key
            found.add(path)
            found |= _key_paths(value, path)
        return found
    if isinstance(payload, list):
        found = set()
        for item in payload:
            found |= _key_paths(item, f"{prefix}[]")
        return found
    return set()


def _compare(clip: Clip, written: Path) -> dict[str, Any]:
    """Diff one replayed file against its committed one. Returns the census, not a verdict."""
    from golf_coach.storage.keypoints_io import load_keypoints

    theirs = load_keypoints(clip.committed)
    ours = load_keypoints(written)
    frames = min(len(theirs.frames), len(ours.frames))

    census: dict[str, Any] = {
        "frames": frames,
        "their_frames": len(theirs.frames),
        "our_frames": len(ours.frames),
        "values": 0,
        "differing": 0,
        "worst": 0.0,
        "examples": [],
    }
    for path, mine, yours in _pairs(ours, theirs, frames):
        census["values"] += 1
        if mine == yours:
            continue
        census["differing"] += 1
        if isinstance(mine, (int, float)) and isinstance(yours, (int, float)):
            census["worst"] = max(census["worst"], abs(float(mine) - float(yours)))
        if len(census["examples"]) < 5:
            census["examples"].append(f"{path}: {yours!r} -> {mine!r}")

    their_keys = _key_paths(json.loads(clip.committed.read_bytes()))
    our_keys = _key_paths(json.loads(written.read_bytes()))
    census["their_keys"] = len(their_keys)
    census["our_keys"] = len(our_keys)
    census["key_diff"] = sorted((their_keys - our_keys) | (our_keys - their_keys))
    return census


def _shortest(clips: list[Clip], count: int) -> list[Clip]:
    """The `count` shortest of these clips, ordered by the size of the keypoints file on disk.

    A proxy for frame count, and a deliberate one: every frame of a keypoints file is the same 33
    landmarks written the same way, so bytes and frames are within a percent of proportional, and
    the alternative is parsing 239 MB of JSON to pick six clips. The sweep prints the frame counts
    it actually posed, so the proxy is visible rather than trusted.
    """
    return sorted(clips, key=lambda clip: clip.committed.stat().st_size)[:count]


class _Usage:
    """Aggregate CPU seconds and peak resident memory of one process tree, sampled while it runs.

    Per-pid maxima *summed* rather than one final reading, because a process that exits between two
    samples takes its CPU time with it — a pool that replaced a dead worker would otherwise report
    less CPU than it spent. Peak RSS is the high-water mark of the **summed live** set, which is the
    number that decides whether a configuration fits on the laptop this ships to: each worker holds
    an interpreter plus a ~30 MB `.task` bundle, and those do not share.
    """

    def __init__(self, pid: int, every: float = 0.25) -> None:
        self.pid = pid
        self.every = every
        self.cpu_by_pid: dict[int, float] = {}
        self.peak_rss = 0
        self.sampled = False
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def __enter__(self) -> _Usage:
        self._thread.start()
        return self

    def __exit__(self, *_: object) -> None:
        self._stop.set()
        self._thread.join(timeout=5)

    @property
    def cpu(self) -> float:
        return sum(self.cpu_by_pid.values())

    def _run(self) -> None:
        try:
            import psutil
        except ImportError:  # reported as "not sampled", never as a zero
            return
        try:
            root = psutil.Process(self.pid)
        except psutil.Error:
            return
        while not self._stop.is_set():
            try:
                tree = [root, *root.children(recursive=True)]
            except psutil.Error:
                break
            resident = 0
            for proc in tree:
                try:
                    times = proc.cpu_times()
                    resident += proc.memory_info().rss
                except psutil.Error:
                    continue  # it exited between the listing and the read
                seen = self.cpu_by_pid.get(proc.pid, 0.0)
                self.cpu_by_pid[proc.pid] = max(seen, times.user + times.system)
                self.sampled = True
            self.peak_rss = max(self.peak_rss, resident)
            self._stop.wait(self.every)


def _sweep(clips: list[Clip], sizes: list[int], python: str | None, variant: str | None) -> int:
    """Run these clips through one pool at each worker count, and report what each one cost."""
    # Read every container once before the first configuration runs. Otherwise the first size pays
    # for pulling ~250 MB off the disk and the later ones read it out of the page cache, which would
    # be charged to the pool size rather than to the ordering.
    warmed = 0
    for clip in clips:
        with clip.video.open("rb") as handle:
            while chunk := handle.read(1 << 22):
                warmed += len(chunk)
    print(f"{len(clips)} clips, {warmed / 1e6:.0f} MB of video, page cache warmed")
    for clip in clips:
        print(f"  {clip.name:<28} {clip.video.name}")

    runs = []
    for size in sizes:
        print(f"\n--- {size} worker(s) ---", flush=True)
        run = _sweep_once(clips, size, python, variant)
        if run is None:
            return 1
        runs.append(run)

    print(
        f"\n{'workers':>7} {'pose s':>8} {'wall s':>8} {'fps':>6} {'cpu s':>9} "
        f"{'cores':>6} {'peak MB':>8} {'speedup':>8}"
    )
    base = runs[0]
    for run in runs:
        cpu = f"{run['cpu']:>9.1f}" if run["sampled"] else f"{'—':>9}"
        cores = f"{run['cpu'] / run['pose']:>6.2f}" if run["sampled"] else f"{'—':>6}"
        peak = f"{run['peak_rss'] / 1e6:>8.0f}" if run["sampled"] else f"{'—':>8}"
        print(
            f"{run['workers']:>7} {run['pose']:>8.1f} {run['wall']:>8.1f} "
            f"{run['frames'] / run['pose']:>6.2f} {cpu} {cores} {peak} "
            f"{base['pose'] / run['pose']:>7.2f}x"
        )
    if not all(run["sampled"] for run in runs):
        print("cpu and peak RSS not sampled — psutil is absent (`pip install -e '.[vision]'`)")

    # The other half of the sweep, and the one P7 could not do: the same clips at every width must
    # come back with the same landmarks. A digest that moved would mean contention changed an
    # answer, which is a much worse finding than a pool size that buys nothing.
    drifted = sorted(
        {
            clip
            for run in runs[1:]
            for clip, mark in run["digests"].items()
            if base["digests"].get(clip) != mark
        }
    )
    frames = {run["frames"] for run in runs}
    print(
        f"{len(base['digests'])} digests, identical across all {len(runs)} configurations"
        if not drifted
        else f"digests differ at a wider pool: {drifted}"
    )
    if len(frames) != 1:
        print(f"frame totals differ between configurations: {sorted(frames)}")
    return 0 if not drifted and len(frames) == 1 else 1


def _sweep_once(
    clips: list[Clip], size: int, python: str | None, variant: str | None
) -> dict[str, Any] | None:
    """One `golf-pose sweep` invocation, timed from out here and sampled while it runs."""
    argv = [str(BINARY), "sweep", *[str(clip.video) for clip in clips], "--workers", str(size)]
    if python:
        argv += ["--python", python]
    if variant:
        argv += ["--variant", variant]

    started = time.monotonic()
    with subprocess.Popen(
        argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
    ) as proc, _Usage(proc.pid) as usage:
        out, err = proc.communicate()
    wall = time.monotonic() - started

    print(out.rstrip())
    if proc.returncode != 0:
        print(f"golf-pose sweep --workers {size} failed: {err.strip()}", file=sys.stderr)
        return None

    digests: dict[str, str] = {}
    frames = 0
    pose = wall
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 5 and parts[1].isdigit():
            digests[parts[0]] = parts[4]
            frames += int(parts[1])
        elif parts[:1] == ["total"]:
            # The binary's own clock, which starts after the handshake — the number that belongs in
            # a throughput comparison, where `wall` also carries N interpreter startups.
            found = re.search(r" in ([0-9.]+)s", line)
            if found:
                pose = float(found.group(1))
    return {
        "workers": size,
        "frames": frames,
        "pose": pose,
        "wall": wall,
        "cpu": usage.cpu,
        "peak_rss": usage.peak_rss,
        "sampled": usage.sampled,
        "digests": digests,
    }


def _report(clips: list[Clip], out_root: Path, python: str | None, variant: str | None) -> int:
    print(
        f"{'clip':<34} {'frames':>6} {'values':>9} {'diff':>5} "
        f"{'worst':>7} {'secs':>7} {'fps':>5}"
    )
    totals = {"clips": 0, "frames": 0, "values": 0, "differing": 0}
    worst = 0.0
    broken: list[str] = []
    started = time.monotonic()

    for clip in clips:
        clip_started = time.monotonic()
        try:
            written, _ = _pose(clip, out_root, python, variant)
        except RuntimeError as failure:
            # A clip the sidecar could not pose is not a clip that disagreed, and conflating the two
            # would let a broken build read as a passing corpus. Named, counted separately, and the
            # run continues — one unreadable container should not cost the other 29.
            print(f"{clip.name:<34} FAILED  {failure}", flush=True)
            broken.append(f"{clip.name}: {failure}")
            continue
        elapsed = time.monotonic() - clip_started
        census = _compare(clip, written)

        totals["clips"] += 1
        totals["frames"] += census["frames"]
        totals["values"] += census["values"]
        totals["differing"] += census["differing"]
        worst = max(worst, census["worst"])
        fps = census["frames"] / elapsed if elapsed else 0.0
        print(
            f"{clip.name:<34} {census['frames']:>6} {census['values']:>9} "
            f"{census['differing']:>5} {census['worst']:>7.3g} {elapsed:>7.1f} {fps:>5.1f}",
            flush=True,
        )
        if census["their_frames"] != census["our_frames"]:
            print(
                f"  frame count: committed {census['their_frames']}, "
                f"replayed {census['our_frames']}",
                flush=True,
            )
            totals["differing"] += 1
        if census["key_diff"]:
            print(f"  key paths differ: {census['key_diff']}", flush=True)
            totals["differing"] += len(census["key_diff"])
        for example in census["examples"]:
            print(f"  {example}", flush=True)

    wall = time.monotonic() - started
    print(
        f"\n{totals['clips']}/{len(clips)} clips compared, {totals['frames']} frames, "
        f"{totals['values']} values, {totals['differing']} differing, "
        f"worst |delta| {worst:.6g}"
    )
    print(f"wall clock {wall / 60:.1f} min ({totals['frames'] / wall:.1f} fps overall)")
    for failure in broken:
        print(f"could not pose {failure}", file=sys.stderr)
    return 1 if broken or totals["differing"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
