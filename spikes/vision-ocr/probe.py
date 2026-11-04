"""M33 Apple Vision spike probe — THROWAWAY. Not part of the `golf_coach` package.

Answers the question `thresholds.md` was committed to before any of this ran: on the photos of
Full Swing's grid screen, does Apple Vision produce boxes that `crates/screen` turns into correct
shots, with no laptop in the path? The plan is `docs/plans/m33-vision-spike.md`, whose section "The
user's decision of 2026-11-04" says what this run may and may not measure.

Python because it is glue over two binaries and a JSON diff, as the earlier spikes' probes are, and
stdlib only. Nothing ships from `spikes/`, so ADR-035 §1 does not reach it.

Subcommands::

    probe.py build-manifest   # writes photos.json from the table below
    probe.py check            # digests and tools; a mismatch is a stop (the plan's rule 9)
    probe.py prepare          # frames/{decoded,lab,vision}/<id>.png and each's sidecar: every mode's input
    probe.py control          # the comparator sees what it must, on committed boxes, with no Vision and no photo
    probe.py measure --config C --set S [--mode M]   # one config's arms: cells, bars, lit table, results/<C>.json
    probe.py failures --config C [--mode M]          # call 11's item 1: each cell not read, and whose it is
    probe.py calibrate --config C                    # §P16: Vision's confidences, the flags, and each threshold's what-if
    probe.py verdict                                 # §P17: call 9's table worked again from results/, held to log.md
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPIKE = ROOT / "spikes" / "vision-ocr"
MANIFEST = SPIKE / "photos.json"
TRUTH = SPIKE / "truth_full_swing.json"

SCREENS = "data/raw/shot_screens"
CAPTURES = "spikes/vision-ocr/captures"

# The six stills as they reached this Mac: the JPEGs Claude Code saved from a chat paste, each beside
# the original file the truth table was read from. Typed from the plan's two digest tables and never
# computed from the files, because a manifest hashed off the disk would make `check` prove nothing.
# (id, the copy's name, the copy's digest, the original's name, the original's digest)
FS_STILLS = (
    (
        "IMG_2889",
        "IMG_2889.jpg",
        "6f22661840a5e698d0b1c72a10fd7214b0b37fa0301f4af4129223f87cff951b",
        "IMG_2889.HEIC",
        "d47e3e9607211a1350c09416d96f2f7e64a3d237a29d6f5a9ba57890f5257d8e",
    ),
    (
        "IMG_2891",
        "IMG_2891.jpg",
        "c13f58c6bb5297daf61a1a489f1e73398f2700b97115c847fad66b1631e35ead",
        "IMG_2891.HEIC",
        "73be98e34cee2eafa22ad8d693c9c1c4c4efac585616f452d6cd5ceda58a15c0",
    ),
    (
        "IMG_2892",
        "IMG_2892.jpg",
        "798395eddca63606a882bb9861b872e81a231a341b4918b5e6a7605d279bce3a",
        "IMG_2892.HEIC",
        "1c4b731de2871303b6387a843e65d893a0c37d059b7ff0cca321eea2f1fc6d04",
    ),
    (
        "IMG_2893",
        "IMG_2893.jpg",
        "240a6cd2caf98e42e90626070f925e6b8bc6ac5d49d1093807d0fb6a22c87d17",
        "IMG_2893.HEIC",
        "204872ef4b6ccd2c7c2730ca14283b43e4a3c5f4e57164373d52c19fb72d9ca8",
    ),
    (
        "IMG_2894",
        "IMG_2894.jpg",
        "4f1e91006ad356fb5d7eb1fd7ef68731216ea9e905756cac0f7ba441453d0a5e",
        "IMG_2894.HEIC",
        "227d9985b337a70248f0e268eea4e73fc3e5860bc0b2418afe6082a4b19c399b",
    ),
    (
        "2026-10-07-1",
        "2026-10-07-1.jpg",
        "f1547903c770c29d9e00548c61b86a5b5fdea2e9ad3b04501b2afbaac072b25d",
        "2026-10-07-1.jpg",
        "96241978f183d70cfbd633802b395362a68ea0b3905324fbbc9a40e18c51b0e1",
    ),
)

# The club-view photo pasted with them. No profile reads it and it has no truth row, so it is not a
# capture, but the decision's table pins it, so `check` holds it all the same.
UNREAD = ("IMG_2888", "IMG_2888.jpg", "fc583423dbf80bb17ef594909138c337aea2a784ae521f7fcd900f8debe21691")

# Not on this Mac, and named so that `check` keeps saying so. Under the decision of 2026-11-04 they
# are not in the manifest, and they are what M33 still needs before it can close.
FS_CLIPS = (
    ("IMG_2889.MOV", "ed2112dbf7f85320d04a1b3d2681853ba5329c989669b8dc1e6095a705c6a2b7"),
    ("IMG_2891.MOV", "0c86193a1af6cf592ced2aa23cca34fe67f19cd1b8eada2601c7019758790815"),
    ("IMG_2892.MOV", "0fa5aa0d2d799d87800693a52d5454ed66ccdf08c293967edc59be9425b13513"),
    ("IMG_2893.MOV", "bf97341db7521dd3b8b50836cc4e90120c192d437b03f18dc6b5903c17b8520a"),
    ("IMG_2894.MOV", "6ed8385c1891a3ba1c5c1db0fd129a77eaf283be88db3395b03fca8fc3943d31"),
)

# Rule 8: every run uses the release builds here, and neither is rebuilt by this script.
VISION_OCR = ROOT / "target" / "m33" / "vision-ocr"
LAB_IMAGES = ROOT / "target" / "m33" / "release" / "vision-spike-lab-images"
GOLF_CORE = ROOT / "target" / "m33" / "release" / "golf-core"

FRAMES = SPIKE / "frames"
DECODED = FRAMES / "decoded"
LAB = FRAMES / "lab"
VISION = FRAMES / "vision"
RUNS = FRAMES / "runs"
RESULTS = SPIKE / "results"

THRESHOLDS = SPIKE / "thresholds.md"
PARSER = ROOT / "crates" / "screen" / "src" / "parser.rs"
PROFILES = ROOT / "crates" / "screen" / "profiles.json"
GRID_VECTOR = ROOT / "spec" / "vectors" / "screen" / "hand" / "full-swing-grid.json"
HD_VECTORS = ("spec/vectors/screen/corpus", "spec/vectors/screen/reference")

# Call 3's modes, in the order the tables print them. `raw` and `vision` are the phone's paths.
MODES = ("raw", "vision", "lab")
PHONE_MODES = ("raw", "vision")

# Call 8. `ScreenInput` refuses a naive timestamp, and nothing here reads the time, so it is fixed
# and two runs of one arm give the same bytes. Every bar is measured at this threshold, whatever P16
# goes on to recommend (call 9).
MIN_CONFIDENCE = 0.6
TIMESTAMP = "2026-11-04T00:00:00Z"

# What `load_image` decodes in process. A HEIC would go through ffmpeg, which rule 6 forbids, and
# `lab-images working` refuses one too.
IN_PROCESS = {".jpg", ".jpeg", ".png"}

# Rule 6's binary: off PATH on purpose, because found by accident it decodes one 896x640 tile of a
# HEIC rather than the photo (crates/ocr/src/decode.rs).
FFMPEG = Path.home() / ".local" / "share" / "golf-coach" / "ffmpeg-7.1"


def truth(photo_id: str) -> dict[str, float]:
    """The twelve stored values of one row, in `ShotData`'s conventions.

    The one place the table's spelling is converted, as `Photo::truth` is in
    `crates/ocr/tests/full_swing.rs`. `launch_direction` is + for R and - for L, `sidespin` + for F
    and - for D. `apex_height` is the printed feet divided by 3.0 and never typed as a decimal: that
    division is the parser's own float, and a comparator holding values to `==` (rule 10) would fail
    a correct read against the nearest two-decimal spelling of it.
    """
    row = json.loads(TRUTH.read_text(encoding="utf-8"))["photos"][photo_id]

    def signed(pair: list, minus: str, plus: str) -> float:
        magnitude, letter = pair
        if letter == minus:
            return -float(magnitude)
        if letter == plus:
            return float(magnitude)
        raise ValueError(f"{photo_id}: {letter} is neither {minus} nor {plus}")

    return {
        "carry_distance": float(row["carry"]),
        "total_distance": float(row["total"]),
        "club_head_speed": float(row["club"]),
        "ball_speed": float(row["ball"]),
        "club_path": float(row["path"]),
        "club_face_angle": float(row["face"]),
        "launch_direction": signed(row["ball_dir"], "L", "R"),
        "launch_angle": float(row["launch"]),
        "apex_height": float(row["apex_ft"]) / 3.0,
        "smash_factor": float(row["smash"]),
        "backspin": float(row["backspin"]),
        "sidespin": signed(row["sidespin"], "D", "F"),
    }


def manifest() -> dict:
    """What `photos.json` holds: the six `fs-still` entries, and only those, by the decision.

    `copy_of` is not in call 1's list of a capture's keys. It is there so that no later phase can
    read a copy as the photo the truth table's digests name, and the verdict's "provisional: pasted
    copies" label has something to rest on other than memory.
    """
    return {
        "_note": (
            "Pasted copies, not the photos as captured: re-encoded and possibly scaled "
            "(docs/plans/m33-vision-spike.md, the user's decision of 2026-11-04). "
            "Written by probe.py build-manifest; do not edit by hand."
        ),
        "captures": [
            {
                "id": photo_id,
                "set": "fs-still",
                "path": f"{CAPTURES}/{name}",
                "sha256": sha256,
                "device": "full_swing",
                "truth": photo_id,
                "copy_of": {"path": f"{SCREENS}/{original}", "sha256": original_sha256},
            }
            for photo_id, name, sha256, original, original_sha256 in FS_STILLS
        ],
    }


def build_manifest(_args: argparse.Namespace) -> int:
    MANIFEST.write_text(json.dumps(manifest(), indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"wrote {MANIFEST.relative_to(ROOT)}: {len(manifest()['captures'])} captures")
    return 0


def held(path: str, sha256: str) -> str:
    """`ok`, `absent` or `MISMATCH`, for a path relative to the repo root."""
    full = ROOT / path
    if not full.is_file():
        return "absent"
    with full.open("rb") as f:
        return "ok" if hashlib.file_digest(f, "sha256").hexdigest() == sha256 else "MISMATCH"


def outstanding() -> list[tuple[str, list[tuple[str, str]]]]:
    """What the decision of 2026-11-04 runs without, by group, as (path, digest).

    HD Golf's come from each vector's `input` and the models' from `crates/ocr/models.json`, so
    there is one copy of each.
    """
    vectors = sorted((ROOT / "spec/vectors/screen/corpus").glob("*.json"))
    vectors += sorted((ROOT / "spec/vectors/screen/reference").glob("*.json"))
    inputs = [json.loads(v.read_text(encoding="utf-8"))["input"] for v in vectors]
    models = json.loads((ROOT / "crates/ocr/models.json").read_text(encoding="utf-8"))["models"]
    return [
        ("the original stills", [(f"{SCREENS}/{s[3]}", s[4]) for s in FS_STILLS]),
        ("the clips", [(f"{SCREENS}/{name}", sha256) for name, sha256 in FS_CLIPS]),
        ("HD Golf", [(i["image_path"], i["image_sha256"]) for i in inputs]),
        ("the OCR models", [(f"data/models/ocr/{m['file']}", m["sha256"]) for m in models.values()]),
    ]


def version(binary: str, flag: str) -> str:
    done = subprocess.run([binary, flag], capture_output=True, text=True, check=False)
    lines = (done.stdout or done.stderr).splitlines()
    return lines[0] if lines else "(no version printed)"


def sw_vers(flag: str) -> str:
    done = subprocess.run(["sw_vers", flag], capture_output=True, text=True, check=False)
    return done.stdout.strip()


def check(_args: argparse.Namespace) -> int:
    """Hold every capture to its digest and say what is missing. Exit 1 on anything that stops a phase.

    **A stop**: a capture missing or differing, any file present with the wrong digest, a stale
    manifest, no `swiftc`, no `cargo`. **Not a stop**: the absent groups, by the decision, and the
    ffmpeg, which only the phases that decision does not run would call.
    """
    stops: list[str] = []

    if not MANIFEST.is_file():
        print(f"check: STOP: no {MANIFEST.name}; run probe.py build-manifest")
        return 1
    captures = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if captures != manifest():
        stops.append(f"{MANIFEST.name} is not what build-manifest writes")

    print("captures (photos.json):")
    matching = 0
    for entry in captures["captures"]:
        state = held(entry["path"], entry["sha256"])
        fields = len(truth(entry["truth"]))
        matching += state == "ok"
        print(f"  {state:9} {entry['id']:13} {entry['path']}  ({fields} truth values)")
        if state != "ok":
            stops.append(f"{entry['id']}: {entry['path']} is {state}")

    photo_id, name, sha256 = UNREAD
    state = held(f"{CAPTURES}/{name}", sha256)
    print(f"not a capture (the club view: no truth row, M33 does not read it):\n  {state:9} {photo_id}")
    if state != "ok":
        stops.append(f"{photo_id}: {CAPTURES}/{name} is {state}")

    print("outstanding (absent is not a stop, by the decision of 2026-11-04):")
    summary = []
    for group, files in outstanding():
        states = [(path, held(path, sha)) for path, sha in files]
        present = sum(s == "ok" for _, s in states)
        print(f"  {group}: {present} of {len(states)} present")
        summary.append(f"{group} {present} of {len(states)}")
        for path, state in states:
            if state != "ok":
                print(f"    {state:9} {path}")
            if state == "MISMATCH":
                stops.append(f"{path} has the wrong digest")

    print("tools:")
    swiftc = shutil.which("swiftc")
    print(f"  swiftc  {swiftc}: {version(swiftc, '--version')}" if swiftc else "  swiftc  MISSING")
    if not swiftc:
        stops.append("no swiftc")

    # A non-interactive shell here does not source the profile that puts cargo on PATH, and rule 3
    # says what to do about that rather than treating it as missing.
    cargo = shutil.which("cargo")
    fallback = Path.home() / ".cargo" / "bin" / "cargo"
    if cargo:
        print(f"  cargo   {cargo}: {version(cargo, '--version')}")
    elif fallback.is_file():
        print(f"  cargo   {fallback}: {version(str(fallback), '--version')}")
        print('          not on PATH; `. "$HOME/.cargo/env"` first (rule 3)')
    else:
        print("  cargo   MISSING")
        stops.append("no cargo")

    if FFMPEG.is_file():
        print(f"  ffmpeg  {FFMPEG}: {version(str(FFMPEG), '-version')}")
    else:
        print(f"  ffmpeg  MISSING at {FFMPEG} (only the clip and lab phases need it)")
    on_path = shutil.which("ffmpeg")
    if on_path:
        print(f"          WARNING: an ffmpeg is on PATH at {on_path}; rule 6 says none may be")
    print(f"  macOS   {sw_vers('-productVersion')} ({sw_vers('-buildVersion')})")

    total = len(captures["captures"])
    if stops:
        print(f"check: STOP: {matching} of {total} captures match")
        for stop in stops:
            print(f"  {stop}")
        return 1
    print(f"check: {matching} of {total} captures match. Outstanding: {'; '.join(summary)}")
    return 0


def run_tool(*command: str) -> str:
    """A binary's stdout. A tool that fails stops the run with its own words, not a half-made set."""
    done = subprocess.run(command, capture_output=True, encoding="utf-8", cwd=ROOT, check=False)
    if done.returncode != 0:
        name = Path(command[0]).name
        raise SystemExit(f"STOP: {name} failed on {command[1:]}: {done.stderr.strip()}")
    return done.stdout


def lab_input(entry: dict) -> Path:
    """The file `working` reads for a capture: JPEG and PNG directly, by call 2.

    The other two routes the plan's P5 names, a HEIC from `frames/decoded/` and a clip from
    `frames/grid/`, are not written, because the decision of 2026-11-04 has nothing to run them on.
    As with P4's `grid-frame`, untested code in the record is worse than a refusal the row that
    brings those files will hit at once.
    """
    path = ROOT / entry["path"]
    if path.suffix.lower() not in IN_PROCESS:
        raise SystemExit(f"prepare: STOP: {entry['id']} is {path.suffix}; this run reads JPEG and PNG only")
    return path


def digest(path: Path) -> str:
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def rectify(entry: dict, decoded: Path, os_build: str) -> dict:
    """`frames/vision/<id>.{png,json}`, cut from `decode`'s PNG, as every Swift step reads (call 2).

    `input_sha256` is that PNG's and `capture_sha256` the file it was decoded from, so that a sidecar
    can be followed back to the capture across the gitignored step between them.
    """
    photo_id = entry["id"]
    image = VISION / f"{photo_id}.png"
    answer = json.loads(run_tool(str(VISION_OCR), "rectify", str(decoded), str(image)))
    sidecar = {
        "id": photo_id,
        "input": decoded.relative_to(ROOT).as_posix(),
        "input_sha256": digest(decoded),
        "capture_sha256": entry["sha256"],
        "image_sha256": digest(image),
        **answer,
        "os": os_build,
    }
    (VISION / f"{photo_id}.json").write_text(
        json.dumps(sidecar, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    return sidecar


def prepare(_args: argparse.Namespace) -> int:
    """Each capture's three inputs, under `frames/` (rule 4) and held to their digests first.

    The sidecar is what a later phase reads instead of asking the binary again: `notes` goes straight
    into a `lab` or `vision` read's input (call 3), and `input_sha256` says which bytes it was cut
    from, since the frames are gitignored and otherwise carry no link to the capture they came from.
    """
    for binary in (VISION_OCR, LAB_IMAGES):
        if not binary.is_file():
            print(f"prepare: STOP: no {binary.relative_to(ROOT)}; the plan's Verify section builds it")
            return 1
    entries = json.loads(MANIFEST.read_text(encoding="utf-8"))["captures"]

    # All of them before any of them, so that one bad copy cannot leave half a set behind.
    sources = {entry["id"]: lab_input(entry) for entry in entries}
    for entry in entries:
        state = held(entry["path"], entry["sha256"])
        if state != "ok":
            print(f"prepare: STOP: {entry['id']}: {entry['path']} is {state} (rule 9)")
            return 1

    os_build = f"macOS {sw_vers('-productVersion')} ({sw_vers('-buildVersion')})"
    print(f"{'id':13} {'decoded':11} {'lab input':10} {'lab image':11} {'quad':22} corners")
    found = 0
    for entry in entries:
        photo_id = entry["id"]
        decoded = DECODED / f"{photo_id}.png"
        answer = json.loads(run_tool(str(VISION_OCR), "decode", str(ROOT / entry["path"]), str(decoded)))

        source = sources[photo_id]
        image = LAB / f"{photo_id}.png"
        LAB.mkdir(parents=True, exist_ok=True)
        lab = json.loads(run_tool(str(LAB_IMAGES), "working", str(source), str(image)))
        sidecar = {
            "id": photo_id,
            "input": source.relative_to(ROOT).as_posix(),
            "input_sha256": entry["sha256"],
            "image_sha256": digest(image),
            "os": os_build,
            **lab,
        }
        (LAB / f"{photo_id}.json").write_text(
            json.dumps(sidecar, indent=2) + "\n", encoding="utf-8", newline="\n"
        )

        quad = lab["quad"]
        found += lab["found"]
        kind = "none: uncropped" if quad is None else str(quad["source"]["kind"])
        corners = "" if quad is None else " ".join(f"({x:.0f},{y:.0f})" for x, y in quad["corners"])
        print(
            f"{photo_id:13} {answer['width']}x{answer['height']:<6} "
            f"{lab['photo']['width']}x{lab['photo']['height']:<5} "
            f"{lab['image']['width']}x{lab['image']['height']:<6} {kind:22} {corners}"
        )
    print(f"prepare: {len(entries)} decoded, {len(entries)} lab images, a quad found on {found}")

    print(f"\n{'id':13} {'vision image':13} {'kept':6} {'of':3} {'area':6} corners (TL TR BR BL, photo pixels)")
    rectified = 0
    for entry in entries:
        vision = rectify(entry, DECODED / f"{entry['id']}.png", os_build)
        rectified += vision["found"]
        kept = f"{vision['confidence']:.3f}" if vision["found"] else "none"
        area = f"{vision['area_fraction']:.3f}" if vision["found"] else "-"
        corners = " ".join(f"({x:.0f},{y:.0f})" for x, y in vision["corners"] or [])
        size = f"{vision['image']['width']}x{vision['image']['height']}"
        print(f"{entry['id']:13} {size:13} {kept:6} {len(vision['candidates']):<3} {area:6} {corners}")
    print(f"prepare: {len(entries)} vision images, a rectangle found on {rectified}")
    return 0


# --- the comparator (calls 8, 9, 12 and 14) ---------------------------------------------------------


def fields() -> tuple[str, ...]:
    """The twelve cells of a Full Swing shot, in the truth table's order, which `truth` owns."""
    return tuple(truth(next(iter(json.loads(TRUTH.read_text(encoding="utf-8"))["photos"]))))


# `ShotData` leaves these two out of its JSON when they are `None` (`skip_serializing_if`,
# crates/contracts/src/shot.rs), where every older field writes `null`. So a sidespin the lit letters
# could not sign is an absent key, and that is call 8's **withheld**, not a fault. P7's controls never
# withheld either, so the first real parse to do it (`IMG_2893`, P9) was what found this.
OMITTED_WHEN_NONE = frozenset({"backspin", "sidespin"})


def stored(shot: dict | None, field: str) -> object:
    """A field of a parsed shot, `None` if the read failed or the field holds nothing.

    Only the two keys above may be absent. Any other missing key still raises, because that would be
    this harness asking for a field the parser does not write, and "withheld" would hide it.
    """
    if shot is None:
        return None
    return shot.get(field) if field in OMITTED_WHEN_NONE else shot[field]


def classify(shot: dict | None, want: dict[str, float]) -> dict:
    """Call 8: one shot's twelve cells against its truth.

    A `null` shot is flagged, because to a golfer a failed read is a re-take, and all of it is
    withheld. A cell that equals the truth is **read** on a flagged shot too: a flag questions the
    shot, and it does not make a right number wrong. `==` on the doubles is rule 10, with no tolerance,
    which `apex_height` depends on, since the truth is the same `feet / 3.0` the parser computes.
    """
    flagged = shot is None or bool(shot["provenance"]["needs_review"])
    cells = {}
    for field, expected in want.items():
        value = stored(shot, field)
        if value is None:
            cell = "withheld"
        elif value == expected:
            cell = "read"
        else:
            cell = "caught" if flagged else "wrong"
        cells[field] = {"cell": cell, "value": value, "truth": expected}
    return {"flagged": flagged, "cells": cells}


# What the record copies from its input or stamps on every shot. Not cells of call 14's report: no
# screen printed them, and four more "read" a shot would only flatter its total. A difference in one
# is the harness's fault, so `classify_vector` refuses it rather than counting it.
ECHOED = ("shot_id", "session_id", "timestamp", "source")

# `crates/ocr/tests/read.rs`'s one `DECLARED_DIFFERENCES` entry (call 14): the vector holds null, and
# the photo shows `CENTER`, by the user's decision recorded there.
DECLARED = {("2026-08-10-1", "impact_position"): "CENTER"}


def classify_vector(vector_id: str, shot: dict | None, expected: dict) -> dict:
    """Call 14: HD Golf's cells, against a vector's `expected.shot` rather than a truth row.

    Call 8's four cells over the keys a screen can supply, plus **extra** where the vector holds null
    and the read a value. A key null on both sides is no cell at all, since most of `ShotData` is not
    on this screen, and counting those as read would drown the ones that are.
    """
    flagged = shot is None or bool(shot["provenance"]["needs_review"])
    cells = {}
    for key, want in expected.items():
        if key == "provenance":
            continue
        value = stored(shot, key)
        if key in ECHOED:
            if shot is not None and value != want:
                raise SystemExit(f"STOP: {vector_id}: {key} is {value!r}, the input's {want!r}: the harness's fault")
            continue
        if value is None and want is None:
            continue
        if value is not None and DECLARED.get((vector_id, key)) == value:
            cell = "read"
        elif value == want:
            cell = "read"
        elif value is None:
            cell = "withheld"
        elif want is None:
            cell = "extra"
        else:
            cell = "caught" if flagged else "wrong"
        cells[key] = {"cell": cell, "value": value, "want": want}
    return {"flagged": flagged, "cells": cells}


def summarize(shots: list[dict]) -> dict:
    """What the bars read, in one place so the synthetic control and a real arm go through the same code."""
    cells = [c for shot in shots for c in shot["cells"].values()]
    totals = {name: sum(c["cell"] == name for c in cells) for name in ("read", "withheld", "wrong", "caught")}
    per_field = {
        field: sum(shot["cells"][field]["cell"] == "read" for shot in shots) for field in shots[0]["cells"]
    }
    return {**totals, "flagged": sum(shot["flagged"] for shot in shots), "per_field": per_field}


def read_bars() -> dict[str, dict[str, int]]:
    """Call 9's limits, parsed out of `thresholds.md` at run time, per set.

    Parsed rather than typed because rule 2 says that file is what the bars are, and a second copy
    here is one that could be edited to pass. A row that no longer parses is a stop, not a default.
    """
    text = THRESHOLDS.read_text(encoding="utf-8")
    rows = {
        "T1": r"\| \*\*T1\*\* wrong \| (\d+) \| (\d+) \|",
        "T2": r"\| \*\*T2\*\* read \| at least (\d+) \| at least (\d+) \|",
        "T3": r"\| \*\*T3\*\* each field read on \| at least (\d+) of (\d+) \| at least (\d+) of (\d+) \|",
        "T4": r"\| \*\*T4\*\* flagged \| at most (\d+) \| at most (\d+) \|",
    }
    found = {}
    for bar, pattern in rows.items():
        matched = re.search(rf"^{pattern}$", text, re.MULTILINE)
        if matched is None:
            raise SystemExit(f"STOP: {THRESHOLDS.name} has no row for {bar}")
        found[bar] = [int(group) for group in matched.groups()]
    return {
        "fs-still": {
            "T1": found["T1"][0],
            "T2": found["T2"][0],
            "T3": found["T3"][0],
            "T4": found["T4"][0],
            "photos": found["T3"][1],
        },
        "fs-clip": {
            "T1": found["T1"][1],
            "T2": found["T2"][1],
            "T3": found["T3"][2],
            "T4": found["T4"][1],
            "photos": found["T3"][3],
        },
    }


def judge(summary: dict, limits: dict[str, int]) -> dict[str, bool]:
    """T1 to T4 for one arm. **caught** is in neither T1's count nor `read`, by call 9's own words."""
    return {
        "T1": summary["wrong"] <= limits["T1"],
        "T2": summary["read"] >= limits["T2"],
        "T3": min(summary["per_field"].values()) >= limits["T3"],
        "T4": summary["flagged"] <= limits["T4"],
    }


def lit_constants() -> dict[str, float]:
    """`LIT_MIN_BRIGHTNESS` and `LIT_BLUE_RATIO`, out of the source call 12 names.

    Read and not typed: a copy that drifted would print a lit table explaining a rule the binary no
    longer applies. `control` holds `is_lit` to the binary as well, on values either side of each.
    """
    text = PARSER.read_text(encoding="utf-8")
    constants = {}
    for name in ("LIT_MIN_BRIGHTNESS", "LIT_BLUE_RATIO"):
        matched = re.search(rf"^pub const {name}: f64 = ([0-9.]+);$", text, re.MULTILINE)
        if matched is None:
            raise SystemExit(f"STOP: {PARSER.relative_to(ROOT)} has no {name}")
        constants[name] = float(matched.group(1))
    return constants


def is_lit(rgb: list[float], constants: dict[str, float]) -> bool:
    """`screen::parser::is_lit`, for the table only. The parser decides; this explains what it decided."""
    red, green, blue = rgb
    floor = min(red, green)
    return floor >= constants["LIT_MIN_BRIGHTNESS"] and blue <= constants["LIT_BLUE_RATIO"] * floor


# Which cell a sign letter signs (`full_swing` in crates/screen/profiles.json, `lit_sign`).
LETTERS = {"L": "launch_direction", "R": "launch_direction", "D": "sidespin", "F": "sidespin"}


def lit_rows(boxes: list[dict], shot: dict | None, constants: dict[str, float]) -> list[dict]:
    """Call 12: every box that is one sign letter exactly, as `lit_sign` spells that, and what it came to.

    Every such box in the image, the tile's or not: only the parser knows which cell a box landed in,
    so `x` and `y` are here for telling the two apart. `margin` is positive on the lit side of
    `LIT_BLUE_RATIO`. `decided` is the sign of the stored value, which is what the letters came to.
    """
    rows = []
    for box in boxes:
        letter = box["text"].strip().upper()
        if letter not in LETTERS:
            continue
        rgb = box.get("rgb")
        field = LETTERS[letter]
        floor = None if rgb is None else min(rgb[0], rgb[1])
        ratio = rgb[2] / floor if floor else None
        value = stored(shot, field)
        rows.append(
            {
                "letter": letter,
                "x": box["x"],
                "y": box["y"],
                "rgb": rgb,
                "ratio": ratio,
                "margin": None if ratio is None else constants["LIT_BLUE_RATIO"] - ratio,
                "lit": None if rgb is None else is_lit(rgb, constants),
                "field": field,
                "decided": None if value is None else math.copysign(1.0, value),
            }
        )
    return rows


def parse_screen(screen_input: dict) -> str:
    """`golf-core parse-screen`'s stdout: a shot, or `null` for a failed read, which exits 0."""
    done = subprocess.run(
        [str(GOLF_CORE), "parse-screen"],
        input=json.dumps(screen_input),
        capture_output=True,
        encoding="utf-8",
        cwd=ROOT,
        check=False,
    )
    if done.returncode != 0:
        raise SystemExit(f"STOP: golf-core parse-screen failed: {done.stderr.strip()}")
    return done.stdout


# --- measure ----------------------------------------------------------------------------------------


def shown(path: Path) -> str:
    return Path(os.path.relpath(path, ROOT)).as_posix()


def read_sidecar(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"measure: STOP: no {shown(path)}; run probe.py prepare")
    return json.loads(path.read_text(encoding="utf-8"))


def mode_image(entry: dict, mode: str) -> tuple[Path, list[str]]:
    """The image a mode reads and the `notes` it goes in with (call 3), held back to the capture.

    The frames are gitignored and nothing else ties them to a digest, so a stale `frames/` would meet
    the truth table with no complaint. Rule 9 asks that only the capture's own pixels meet it, and
    each sidecar records what its image was cut from, so a mismatch here is the same stop.
    """
    photo_id = entry["id"]
    rectified = read_sidecar(VISION / f"{photo_id}.json")
    if rectified["capture_sha256"] != entry["sha256"]:
        raise SystemExit(f"measure: STOP: {photo_id}: {shown(VISION)} is not from this capture; run probe.py prepare")
    if mode == "raw":
        image, notes, expected = DECODED / f"{photo_id}.png", [], rectified["input_sha256"]
    elif mode == "vision":
        image, notes, expected = VISION / f"{photo_id}.png", rectified["notes"], rectified["image_sha256"]
    else:
        working = read_sidecar(LAB / f"{photo_id}.json")
        if working["input_sha256"] != entry["sha256"]:
            raise SystemExit(f"measure: STOP: {photo_id}: {shown(LAB)} is not from this capture; run probe.py prepare")
        image, notes, expected = LAB / f"{photo_id}.png", working["notes"], working["image_sha256"]
    if not image.is_file() or digest(image) != expected:
        raise SystemExit(f"measure: STOP: {shown(image)} is not what its sidecar says; run probe.py prepare")
    return image, notes


def screen_input(entry: dict, boxes: list[dict], notes: list[str]) -> dict:
    """Call 8's input. The paths and digest are the capture's: the frames it was read from are derived."""
    return {
        "device": entry["device"],
        "boxes": boxes,
        "notes": notes,
        "shot_id": entry["id"],
        "session_id": "m33",
        "timestamp": TIMESTAMP,
        "image_sha256": entry["sha256"],
        "image_path": entry["path"],
        "min_confidence": MIN_CONFIDENCE,
    }


def run_arm(entries: list[dict], mode: str, config: str, limits: dict[str, int], constants: dict) -> dict:
    """One arm: Vision, then the parser, then the cells, for every capture of the set.

    Each boxes file holds Vision's stdout as printed, so that P9's two runs can be compared byte for
    byte and `repeat` says so here. `frames/runs/` is overwritten by the next run of the arm.
    """
    runs = RUNS / config / mode
    runs.mkdir(parents=True, exist_ok=True)
    shots = {}
    os_build = revision = None
    for entry in entries:
        photo_id = entry["id"]
        image, notes = mode_image(entry, mode)
        relative = shown(image)
        text = run_tool(str(VISION_OCR), "read", relative, "--config", config)
        answer = json.loads(text)
        if answer["config"] != config:
            raise SystemExit(f"measure: STOP: asked for {config}, and vision-ocr printed {answer['config']}")
        os_build, revision = answer["os"], answer["revision"]

        boxes_file = runs / f"{photo_id}.boxes.json"
        repeat = "first"
        if boxes_file.is_file():
            repeat = "same" if boxes_file.read_text(encoding="utf-8") == text else "DIFFERS"
        boxes_file.write_text(text, encoding="utf-8", newline="\n")

        parsed = parse_screen(screen_input(entry, answer["boxes"], notes))
        (runs / f"{photo_id}.shot.json").write_text(parsed, encoding="utf-8", newline="\n")
        shot = json.loads(parsed)

        result = classify(shot, truth(entry["truth"]))
        provenance = None if shot is None else shot["provenance"]
        shots[photo_id] = {
            **result,
            "image": relative,
            "image_sha256": digest(image),
            "notes": notes,
            "boxes": len(answer["boxes"]),
            "boxes_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
            "repeat": repeat,
            "failed_read": shot is None,
            "parse_confidence": None if provenance is None else provenance["parse_confidence"],
            "parser_version": None if provenance is None else provenance["parser_version"],
            "warnings": [] if provenance is None else provenance["warnings"],
            "raw_fields": {} if provenance is None else provenance["raw_fields"],
            "lit": lit_rows(answer["boxes"], shot, constants),
        }
    summary = summarize(list(shots.values()))
    bars = judge(summary, limits)
    return {
        "os": os_build,
        "revision": revision,
        "summary": summary,
        "bars": bars,
        "pass": all(bars.values()),
        "shots": shots,
    }


def tile_labels() -> dict[str, str]:
    """Each stored field's tile, for naming what the parser saw under it, from the profile itself."""
    profiles = json.loads(PROFILES.read_text(encoding="utf-8"))["profiles"]
    full_swing = next(p for p in profiles if p["device"] == "full_swing")
    return {f["target"]: f["label"] for f in full_swing["fields"] if f["target"]}


def number(value: float | None, spec: str) -> str:
    return "-" if value is None else format(value, spec)


def print_lit(shots: dict, constants: dict[str, float]) -> None:
    print(
        f"lit, each box that is one letter (LIT_MIN_BRIGHTNESS {constants['LIT_MIN_BRIGHTNESS']}, "
        f"LIT_BLUE_RATIO {constants['LIT_BLUE_RATIO']}, {PARSER.relative_to(ROOT).as_posix()}):"
    )
    print(
        f"  {'id':13} {'letter':6} {'x,y':11} {'r':>7} {'g':>7} {'b':>7} {'b/min':>6} {'margin':>7} {'lit':5} "
        "sign decided"
    )
    any_row = False
    for photo_id, shot in shots.items():
        for row in shot["lit"]:
            any_row = True
            rgb = row["rgb"] or [None, None, None]
            sign = {None: "none", 1.0: "+", -1.0: "-"}[row["decided"]]
            truth_value = shot["cells"][row["field"]]["truth"]
            print(
                f"  {photo_id:13} {row['letter']:6} {row['x']:>5.0f},{row['y']:<5.0f} "
                f"{number(rgb[0], '7.2f'):>7} {number(rgb[1], '7.2f'):>7} {number(rgb[2], '7.2f'):>7} "
                f"{number(row['ratio'], '6.3f'):>6} {number(row['margin'], '+7.3f'):>7} {str(row['lit']):5} "
                f"{row['field']} {sign} (truth {'+' if truth_value > 0 else '-'})"
            )
    if not any_row:
        print("  (no box is one letter)")


def print_arm(
    capture_set: str, mode: str, config: str, arm: dict, limits: dict[str, int], constants: dict
) -> None:
    """The tables P9 asks for, one arm: cells per photo, totals, per field, what was not read, bars, lit."""
    summary = arm["summary"]
    print(f"\n--- {capture_set}, {mode}: {config}, Vision revision {arm['revision']}, {arm['os']} ---")
    print(
        f"{'id':13} {'read':>4} {'withheld':>8} {'wrong':>5} {'caught':>6} {'flagged':7} {'conf':>5} {'boxes':>5} "
        "repeat"
    )
    for photo_id, shot in arm["shots"].items():
        counts = summarize([shot])
        flag = "FAILED" if shot["failed_read"] else "yes" if shot["flagged"] else "no"
        print(
            f"{photo_id:13} {counts['read']:>4} {counts['withheld']:>8} {counts['wrong']:>5} {counts['caught']:>6} "
            f"{flag:7} {number(shot['parse_confidence'], '.3f'):>5} {shot['boxes']:>5} {shot['repeat']}"
        )
    print(
        f"{'total':13} {summary['read']:>4} {summary['withheld']:>8} {summary['wrong']:>5} {summary['caught']:>6} "
        f"{summary['flagged']:<7}"
    )

    photos = len(arm["shots"])
    print(f"read per field, of {photos}:")
    for field, count in summary["per_field"].items():
        print(f"  {field:18} {count}")

    labels = tile_labels()
    misses = [
        (photo_id, field, cell, shot)
        for photo_id, shot in arm["shots"].items()
        for field, cell in shot["cells"].items()
        if cell["cell"] != "read"
    ]
    print(f"not read ({len(misses)}):")
    for photo_id, field, cell, shot in misses:
        tile = shot["raw_fields"].get(labels[field])
        print(
            f"  {photo_id:13} {field:18} {cell['cell']:8} value {cell['value']!s:>20} truth {cell['truth']!s:>20} "
            f"tile {tile!r}"
        )
    flagged = [(photo_id, shot) for photo_id, shot in arm["shots"].items() if shot["flagged"]]
    if flagged:
        print("why flagged:")
        for photo_id, shot in flagged:
            why = "a failed read" if shot["failed_read"] else "; ".join(shot["warnings"]) or "(no warning)"
            print(f"  {photo_id:13} {why}")

    bars = arm["bars"]
    per_field = summary["per_field"]
    lowest = min(per_field, key=per_field.get)
    verdict = {bar: "pass" if bars[bar] else "MISS" for bar in bars}
    print("bars:")
    print(f"  T1 wrong {summary['wrong']}, at most {limits['T1']}: {verdict['T1']}")
    print(f"  T2 read {summary['read']}, at least {limits['T2']}: {verdict['T2']}")
    print(
        f"  T3 each field read on, lowest {per_field[lowest]} ({lowest}), "
        f"at least {limits['T3']} of {limits['photos']}: {verdict['T3']}"
    )
    print(f"  T4 flagged {summary['flagged']}, at most {limits['T4']}: {verdict['T4']}")
    print(f"arm: {'PASS' if arm['pass'] else 'MISS'}")
    print_lit(arm["shots"], constants)


def measure(args: argparse.Namespace) -> int:
    """One config over one set's modes: run, classify, print, and record in `results/<config>.json`.

    A miss is a result, so it exits 0. It exits 1 only where something stopped the measurement: rule
    9's digests, a frame that does not lead back to its capture, or a set with nothing to read.
    Everything is checked before anything is read, so a stop leaves no half-measured arm behind.
    """
    config, capture_set = args.config, args.capture_set
    for binary in (VISION_OCR, GOLF_CORE):
        if not binary.is_file():
            print(f"measure: STOP: no {binary.relative_to(ROOT)}; the plan's Verify section builds it")
            return 1
    entries = [
        entry
        for entry in json.loads(MANIFEST.read_text(encoding="utf-8"))["captures"]
        if entry["set"] == capture_set
    ]
    if not entries:
        # The decision of 2026-11-04 leaves `fs-clip` and `hd` out of the manifest. Their modes are not
        # written either (P4 wrote no grid frames, and `hd` has no bars), as with P5's `lab_input`:
        # the row that brings those files hits this at once, rather than untested code guessing.
        print(f"measure: STOP: {MANIFEST.name} holds no capture of set {capture_set!r} (the decision of 2026-11-04)")
        return 1
    all_bars = read_bars()
    if capture_set not in all_bars:
        print(f"measure: STOP: {capture_set!r} has no bars in {THRESHOLDS.name}")
        return 1
    limits = all_bars[capture_set]
    if len(entries) != limits["photos"]:
        print(f"measure: STOP: {len(entries)} captures of {capture_set}, and the bars are for {limits['photos']}")
        return 1

    for entry in entries:
        state = held(entry["path"], entry["sha256"])
        if state != "ok":
            print(f"measure: STOP: {entry['id']}: {entry['path']} is {state} (rule 9)")
            return 1
    modes = (args.mode,) if args.mode else MODES
    for mode in modes:
        for entry in entries:
            mode_image(entry, mode)

    constants = lit_constants()
    # On data rather than memory, as `copy_of` was added for: a capture whose digest is not its
    # original's is what makes this provisional.
    pasted = any("copy_of" in e and e["copy_of"]["sha256"] != e["sha256"] for e in entries)
    label = "provisional: pasted copies" if pasted else None
    arms = {}
    for mode in modes:
        arms[mode] = run_arm(entries, mode, config, limits, constants)
        print_arm(capture_set, mode, config, arms[mode], limits, constants)

    # Merged, because P9, P10 and P11 each add a set to one config's file, and `--mode` adds one arm.
    RESULTS.mkdir(parents=True, exist_ok=True)
    results_file = RESULTS / f"{config}.json"
    record = {"config": config, "min_confidence": MIN_CONFIDENCE, "sets": {}}
    if results_file.is_file():
        record = json.loads(results_file.read_text(encoding="utf-8"))
    record["thresholds_sha256"] = digest(THRESHOLDS)
    held_set = record["sets"].setdefault(capture_set, {"arms": {}})
    held_set.update({"label": label, "limits": limits})
    held_set["arms"].update(arms)
    results_file.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8", newline="\n")

    print(f"\nmeasure {config} {capture_set}" + (f" ({label})" if label else "") + ":")
    for mode in modes:
        print(f"  {mode:7} {'PASS' if arms[mode]['pass'] else 'MISS'}")
    print(f"wrote {shown(results_file)}")
    return 0


# --- failures (call 11's condition and its item 1) ---------------------------------------------------


# Each cell's column in the truth table: `failures` asks what the screen printed, so it reads the table's
# own spelling, unsigned and in feet, where `truth` has already turned it into `ShotData`'s.
PRINTED_COLUMNS = {
    "carry_distance": "carry",
    "total_distance": "total",
    "club_head_speed": "club",
    "ball_speed": "ball",
    "club_path": "path",
    "club_face_angle": "face",
    "launch_direction": "ball_dir",
    "launch_angle": "launch",
    "apex_height": "apex_ft",
    "smash_factor": "smash",
    "backspin": "backspin",
    "sidespin": "sidespin",
}

# `parser.rs`'s number pattern, with the typographic minuses Vision could return beside a hyphen. The
# parser's pattern takes no such minus, and that is why this one does: `-1.7` returned with U+2212 is in
# Vision's boxes and read wrongly, which is the very thing item 1 asks about.
NUMBER = re.compile("[-+\u2212\u2013\u2014]?\\d[\\d,]*(?:\\.\\d+)?")


def printed(photo_id: str, field: str) -> float:
    """The number the screen shows for a cell, as the table spells it, and without its lit letter."""
    column = json.loads(TRUTH.read_text(encoding="utf-8"))["photos"][photo_id][PRINTED_COLUMNS[field]]
    return float(column[0] if isinstance(column, list) else column)


def numbers_in(text: str) -> list[float]:
    found = NUMBER.findall(text)
    return [float(re.sub("[\u2212\u2013\u2014]", "-", number).replace(",", "")) for number in found]


def sign_letters(field: str) -> str:
    """The pair that signs a cell (`LETTERS`, which the profile's `lit_sign` fields are), else nothing."""
    return "".join(sorted(letter for letter, signed in LETTERS.items() if signed == field))


def boxes_with_letter(boxes: list[dict], letters: str) -> list[dict]:
    """Boxes that hold a letter of the pair as a token of its own: `D`, `638 D` or `638D`, and not `DD`.

    Wider than `lit_sign`, which takes a box that is the letter exactly, because a letter merged into
    its number is present in Vision's boxes and read not at all, and item 1 has to call that fixable.
    A box with no `rgb` does not count: the letter decides by its colour, and that is the host's to
    measure, so no parser rule can recover what the host did not hand it.
    """
    pattern = re.compile(rf"(?<![A-Z])[{letters}](?![A-Z])")
    return [box for box in boxes if box.get("rgb") is not None and pattern.search(box["text"].upper())]


def whose(photo_id: str, field: str, boxes: list[dict], raw_fields: dict[str, str]) -> dict:
    """Call 11's item 1, for one cell not read: is what the screen printed in Vision's boxes at all?

    **Anywhere on the photo counts, not only under the label.** A label Vision garbled leaves no tile
    to look in, and its number is then the plainest parser-fixable miss there is. So this errs towards
    calling a cell fixable, and a number that merely recurs elsewhere is laid out in the row
    (`in_tile`, and where each box sits), to be read rather than trusted. Item 1 is necessary and not
    sufficient: the other seven of call 11's checks are what would decide a rule.

    A signed cell needs a letter of its pair as well as its number. Its magnitude alone cannot be
    stored, and item 3 says nothing but the lit letters may sign it, so with neither letter returned
    the number is present and the *value* is still not, and the miss is the recognizer's.
    """
    number = printed(photo_id, field)
    holding = [box for box in boxes if number in numbers_in(box["text"])]
    tile = raw_fields.get(tile_labels()[field])
    letters = sign_letters(field)
    lettered = boxes_with_letter(boxes, letters) if letters else []
    return {
        "printed": number,
        "letters": letters,
        "tile": tile,
        "in_tile": tile is not None and number in numbers_in(tile),
        "holding": holding,
        "lettered": lettered,
        "fixable": bool(holding) and (not letters or bool(lettered)),
    }


def why_not_fixable(row: dict, boxes: list[dict]) -> str:
    if not boxes:
        return "this image gave no boxes"
    if not row["holding"]:
        return f"{row['printed']:g} is in no box"
    return f"{row['printed']:g} is present and no box holds {' or '.join(row['letters'])}"


def run_files(config: str, mode: str, photo_id: str) -> tuple[Path, Path]:
    return RUNS / config / mode / f"{photo_id}.boxes.json", RUNS / config / mode / f"{photo_id}.shot.json"


def arm_failures(config: str, mode: str, arm: dict) -> list[dict]:
    """Each cell not read on one arm, from the run's own files, each held to what `results/` recorded.

    `frames/` is gitignored and overwritten by the next run, so a stale file would answer for a
    measurement it did not make. Vision does not run here: re-reading would rewrite `repeat`, P9's
    record that two runs matched, and what is being asked about is the measurement as it stands.
    """
    rows = []
    for photo_id, shot in arm["shots"].items():
        boxes_file, shot_file = run_files(config, mode, photo_id)
        if not boxes_file.is_file() or not shot_file.is_file():
            raise SystemExit(f"failures: STOP: no {shown(boxes_file)} or its shot; run probe.py measure")
        text = boxes_file.read_text(encoding="utf-8")
        if hashlib.sha256(text.encode("utf-8")).hexdigest() != shot["boxes_sha256"]:
            raise SystemExit(f"failures: STOP: {shown(boxes_file)} is not the boxes {config}'s results recorded")
        parsed = json.loads(shot_file.read_text(encoding="utf-8"))
        if classify(parsed, truth(photo_id))["cells"] != shot["cells"]:
            raise SystemExit(f"failures: STOP: {shown(shot_file)} does not classify as {config}'s results do")
        boxes = json.loads(text)["boxes"]
        for field, cell in shot["cells"].items():
            if cell["cell"] != "read":
                rows.append(
                    {
                        "id": photo_id,
                        "field": field,
                        "cell": cell["cell"],
                        "boxes": boxes,
                        **whose(photo_id, field, boxes, shot["raw_fields"]),
                    }
                )
    return rows


def failures(args: argparse.Namespace) -> int:
    """Steps 1 and 2 of P13: call 11's condition, then each cell not read and whether it is fixable.

    The default arms are the target's, the phone modes. `--mode lab` is a diagnostic, because call 11's
    condition and the target name only those two. It writes nothing: `results/` is the measurement, and
    this is a reading of it. A fixable cell exits 0 like any other result. It exits 1 only where the
    reading would rest on something other than the measurement.
    """
    config = args.config
    results_file = RESULTS / f"{config}.json"
    if not results_file.is_file():
        print(f"failures: STOP: no {shown(results_file)}; run probe.py measure")
        return 1
    record = json.loads(results_file.read_text(encoding="utf-8"))
    if record["thresholds_sha256"] != digest(THRESHOLDS):
        print(f"failures: STOP: {THRESHOLDS.name} is not the file {shown(results_file)} was judged against (rule 2)")
        return 1

    all_bars = read_bars()
    modes = (args.mode,) if args.mode else PHONE_MODES
    fixable: list[dict] = []
    total = 0
    condition = {}
    for capture_set in ("fs-still", "fs-clip"):
        held_set = record["sets"].get(capture_set)
        if held_set is None:
            print(f"\n{capture_set}: not measured")
            continue
        arms = held_set["arms"]
        label = f" ({held_set['label']})" if held_set["label"] else ""
        print(f"\n{capture_set}{label}, {config}:")

        # Judged again rather than taken from `pass`, so that what this phase ticks on is the bars in
        # thresholds.md applied to the cells, and a results file that disagreed would stop it.
        for mode, arm in arms.items():
            bars = judge(summarize(list(arm["shots"].values())), all_bars[capture_set])
            if bars != arm["bars"] or all(bars.values()) != arm["pass"]:
                print(f"failures: STOP: {capture_set} {mode}: its cells do not judge as {shown(results_file)} says")
                return 1
            print(f"  {mode:7} {'PASS' if arm['pass'] else 'MISS'}, read {arm['summary']['read']}")
        condition[capture_set] = not any(arms[mode]["pass"] for mode in PHONE_MODES)

        for mode in modes:
            rows = arm_failures(config, mode, arms[mode])
            total += len(rows)
            fixable += [{**row, "mode": mode} for row in rows if row["fixable"]]
            target = "a target arm" if mode in PHONE_MODES else "diagnostic, not a target arm"
            print(f"\n  {mode} ({target}), not read ({len(rows)}):")
            print(f"    {'id':13} {'field':17} {'cell':8} {'printed':>8} {'tile':20} {'in tile':7} whose")
            for row in rows:
                verdict = "PARSER-FIXABLE" if row["fixable"] else f"not fixable: {why_not_fixable(row, row['boxes'])}"
                print(
                    f"    {row['id']:13} {row['field']:17} {row['cell']:8} {row['printed']:>8g} "
                    f"{row['tile']!r:20} {str(row['in_tile']):7} {verdict}"
                )
                where = [f"{box['text']!r} at {box['x']:.0f},{box['y']:.0f}" for box in row["holding"]]
                where += [f"letter box {box['text']!r} at {box['x']:.0f},{box['y']:.0f}" for box in row["lettered"]]
                if where:
                    print(f"      {'; '.join(where)}")

    print(f"\ncall 11's condition (a miss on both phone modes), {config}:")
    for capture_set in ("fs-still", "fs-clip"):
        state = condition.get(capture_set)
        print(f"  {capture_set}: {'not measured, which is not a miss' if state is None else 'holds' if state else 'does not hold'}")
    print(f"parser-fixable on {', '.join(modes)}: {len(fixable)} of {total} cells not read")
    for row in fixable:
        print(f"  {row['mode']} {row['id']} {row['field']}")
    return 0


# --- calibrate (P16) ---------------------------------------------------------------------------------


# §P16's what-if, and the four its rule chooses among. 0.5 is only looked at: under the 0.6 the bars were
# measured at it can only flag less, so the rule does not offer it.
WHAT_IF = (0.5, 0.6, 0.7, 0.8, 0.9)
RUNGS = (0.6, 0.7, 0.8, 0.9)


def score_weights() -> dict[str, float]:
    """`score`'s three weights, out of the source, as `lit_constants` reads its two. Only explained here, never moved."""
    text = PARSER.read_text(encoding="utf-8")
    weights = {}
    for name in ("W_LABELS", "W_VALUES", "W_OCR"):
        matched = re.search(rf"^const {name}: f64 = ([0-9.]+);$", text, re.MULTILINE)
        if matched is None:
            raise SystemExit(f"STOP: {PARSER.relative_to(ROOT)} has no {name}")
        weights[name] = float(matched.group(1))
    return weights


def read_at(base: dict, boxes: list[dict], min_confidence: float) -> dict | None:
    """The parser with the caller's threshold changed, which is the only thing this phase changes (`ScreenInput`)."""
    return json.loads(parse_screen({**base, "boxes": boxes, "min_confidence": min_confidence}))


def all_at(boxes: list[dict], confidence: float) -> list[dict]:
    return [{**box, "confidence": confidence} for box in boxes]


def averaged_boxes(base: dict, boxes: list[dict], ocr_weight: float) -> list[int]:
    """Which boxes `score` averaged, by index, asked of the parser rather than worked out again here.

    `used` is local to `parse_screen`, no output names it, and rule 5 keeps `crates/` shut. So move one
    box's confidence and watch `parse_confidence`: a Python copy of `build_cells` would be a second
    parser, whose agreement with the first nothing here could certify. The move is at least half a unit
    on a mean of at most `len(boxes)`, and it has to clear the three decimals the record is rounded to,
    which is the check. `confidence_terms` then holds the answer to the arithmetic.
    """
    if ocr_weight * 0.5 / max(len(boxes), 1) < 0.001:
        raise SystemExit(f"calibrate: STOP: moving one box of {len(boxes)} would not show in three decimals")

    def confidence_of(trial: list[dict]) -> float:
        return read_at(base, trial, 0.0)["provenance"]["parse_confidence"]

    before = confidence_of(boxes)
    if before <= 0.0:
        raise SystemExit("calibrate: STOP: a score clamped at 0 would hide the movement")
    return [
        index
        for index, box in enumerate(boxes)
        if confidence_of([*boxes[:index], {**box, "confidence": 0.0 if box["confidence"] >= 0.5 else 1.0}, *boxes[index + 1:]])
        != before
    ]


def without_score(shot: dict) -> dict:
    provenance = {k: v for k, v in shot["provenance"].items() if k != "parse_confidence"}
    return {**shot, "provenance": provenance}


def confidence_terms(base: dict, boxes: list[dict], weights: dict[str, float]) -> dict:
    """One shot's `parse_confidence` taken apart by the binary, with each part held to `score`'s arithmetic.

    Every box's confidence at 0 leaves the two coverage terms less any penalty, and what that takes off
    the shot's own is the OCR term. Each side is rounded to three decimals, so the term is good to 0.001,
    and a difference beyond that from `W_OCR` times the mean of `averaged_boxes` is a stop: it would mean
    that list is not what the parser averaged. Comparing the rest of the shot is the measurement that a
    box's confidence reaches the score and nothing else, no value and no flag at a threshold of 0.
    """
    shot, floor, ceiling = (
        read_at(base, trial, 0.0) for trial in (boxes, all_at(boxes, 0.0), all_at(boxes, 1.0))
    )
    if not without_score(shot) == without_score(floor) == without_score(ceiling):
        raise SystemExit(f"calibrate: STOP: {base['shot_id']}: a box's confidence moved something other than the score")
    used = averaged_boxes(base, boxes, weights["W_OCR"])
    mean = sum(boxes[index]["confidence"] for index in used) / len(used)
    own = shot["provenance"]["parse_confidence"]
    coverage = floor["provenance"]["parse_confidence"]
    full_term = ceiling["provenance"]["parse_confidence"] - coverage
    if abs(weights["W_OCR"] * mean - (own - coverage)) > 0.001 + 1e-9 or abs(weights["W_OCR"] - full_term) > 0.001 + 1e-9:
        raise SystemExit(f"calibrate: STOP: {base['shot_id']}: the boxes found do not add up to its parse_confidence")
    return {"parse_confidence": own, "coverage": coverage, "ocr_term": round(own - coverage, 3), "mean": mean, "used": used}


def flag_cause(base: dict, boxes: list[dict], min_confidence: float) -> str | None:
    """Why a shot is flagged at this threshold, which no output says (P7: a low confidence adds no warning).

    `validate_parse` flags on `penalty > 0.0 || confidence < min_confidence`, and a confidence is clamped
    at 0, so at a threshold of 0 only the penalty can fire. Physics is what is still flagged there.
    """
    shot = read_at(base, boxes, min_confidence)
    if shot is None:
        return "failed read"
    if not shot["provenance"]["needs_review"]:
        return None
    return "physics" if read_at(base, boxes, 0.0)["provenance"]["needs_review"] else "confidence alone"


def what_if_arm(reads: list[tuple[str, dict, list[dict], dict]], min_confidence: float) -> dict:
    """One arm re-parsed at one threshold and classified afresh, since **wrong** and **caught** turn on the flag."""
    row: dict = {"flagged": [], "causes": {}, "wrong": [], "caught": [], "read": 0}
    for photo_id, base, boxes, want in reads:
        result = classify(read_at(base, boxes, min_confidence), want)
        if result["flagged"]:
            row["flagged"].append(photo_id)
            row["causes"][photo_id] = flag_cause(base, boxes, min_confidence)
        for field, cell in result["cells"].items():
            if cell["cell"] in ("wrong", "caught"):
                row[cell["cell"]].append((photo_id, field, cell["value"], cell["truth"]))
            row["read"] += cell["cell"] == "read"
    return row


def rung_holds(arms: dict[str, dict], ceiling: int) -> bool:
    """§P16's test on one threshold: on each phone mode, every shot holding a differing cell is flagged, within T4."""
    return all(not arms[mode]["wrong"] and len(arms[mode]["flagged"]) <= ceiling for mode in PHONE_MODES)


def recommend(table: dict[float, dict[str, dict]], ceiling: int) -> float:
    """§P16's rule: the lowest of the four that holds, and 0.6 if none does."""
    return next((rung for rung in RUNGS if rung_holds(table[rung], ceiling)), RUNGS[0])


def tiles_to_flag(weights: dict[str, float], tiles: int, rung: float) -> int | None:
    """How many tiles' labels a shot must lose to fall under a threshold, with every box at 1.00. `score`'s sum, by hand."""
    for lost in range(tiles + 1):
        if (weights["W_LABELS"] + weights["W_VALUES"]) * (tiles - lost) / tiles + weights["W_OCR"] < rung:
            return lost
    return None


def calibrate_reads(record: dict, capture_set: str, mode: str, config: str) -> list[tuple[str, dict, list[dict], dict]]:
    """An arm's inputs, from the committed boxes, each parse held to what `results/` recorded at its threshold.

    The committed boxes rather than `frames/`, which is gitignored, and Vision does not run: re-reading
    would rewrite `repeat`, and this phase reads the measurement. Holding the parse here is what makes
    the what-if's row at 0.6 the bars' own row and not a neighbouring one.
    """
    entries = {
        entry["id"]: entry
        for entry in json.loads(MANIFEST.read_text(encoding="utf-8"))["captures"]
        if entry["set"] == capture_set
    }
    reads = []
    for photo_id, shot in record["sets"][capture_set]["arms"][mode]["shots"].items():
        path = SPIKE / "boxes" / config / mode / f"{photo_id}.json"
        if not path.is_file():
            raise SystemExit(f"calibrate: STOP: no {shown(path)}")
        text = path.read_text(encoding="utf-8")
        if hashlib.sha256(text.encode("utf-8")).hexdigest() != shot["boxes_sha256"]:
            raise SystemExit(f"calibrate: STOP: {shown(path)} is not the boxes {config}'s results recorded")
        boxes = json.loads(text)["boxes"]
        base = screen_input(entries[photo_id], boxes, shot["notes"])
        want = truth(entries[photo_id]["truth"])
        again = read_at(base, boxes, record["min_confidence"])
        result = classify(again, want)
        provenance = None if again is None else again["provenance"]
        if (
            result["cells"] != shot["cells"]
            or result["flagged"] != shot["flagged"]
            or (None if provenance is None else provenance["parse_confidence"]) != shot["parse_confidence"]
            or ([] if provenance is None else provenance["warnings"]) != shot["warnings"]
        ):
            raise SystemExit(f"calibrate: STOP: {mode} {photo_id}: the parse is not what {config}'s results recorded")
        reads.append((photo_id, base, boxes, want))
    return reads


def calibrate(args: argparse.Namespace) -> int:
    """ADR-014's question: do Vision's coarse confidences change `needs_review`, and what should the phone pass?

    It prints and writes nothing, as `failures` does: `results/` is the measurement and this is a reading
    of it. The bars stay as measured at 0.6, whatever it recommends (call 9). Exits 1 only where the
    reading would rest on something other than the measurement.
    """
    config = args.config
    results_file = RESULTS / f"{config}.json"
    if not GOLF_CORE.is_file():
        print(f"calibrate: STOP: no {GOLF_CORE.relative_to(ROOT)}; the plan's Verify section builds it")
        return 1
    if not results_file.is_file():
        print(f"calibrate: STOP: no {shown(results_file)}; run probe.py measure")
        return 1
    record = json.loads(results_file.read_text(encoding="utf-8"))
    if record["thresholds_sha256"] != digest(THRESHOLDS):
        print(f"calibrate: STOP: {THRESHOLDS.name} is not the file {shown(results_file)} was judged against (rule 2)")
        return 1
    weights = score_weights()
    print(f"calibrate {config}, the bars measured at {record['min_confidence']}; {weights} from {shown(PARSER)}")
    print("the lab's reader: not measured (P8), so no column against it; `lab` below is Vision on the lab's image")

    # Only the set this run measured is wired (the decision of 2026-11-04). A clip's id is its still's, so
    # `boxes/` would have to tell the two apart first, and the row that brings them hits this at once,
    # as P5's `lab_input` does, rather than untested code guessing.
    for capture_set in ("fs-clip", "hd"):
        if capture_set in record["sets"]:
            print(f"calibrate: STOP: {capture_set} is measured, and this reads only fs-still until its row wires it")
            return 1
        print(f"{capture_set}: not measured")
    capture_set = "fs-still"
    if capture_set not in record["sets"]:
        print(f"calibrate: STOP: {shown(results_file)} holds no {capture_set}; run probe.py measure")
        return 1
    held_set = record["sets"][capture_set]
    label = f" ({held_set['label']})" if held_set["label"] else ""
    ceiling = read_bars()[capture_set]["T4"]
    arms = {mode: calibrate_reads(record, capture_set, mode, config) for mode in MODES if mode in held_set["arms"]}

    print(f"\n{capture_set}{label}\n\n1. Vision's box confidences")
    print(f"  {'mode':7} {'boxes':>5} {'distinct':>8} histogram")
    every: dict[float, int] = {}
    terms: dict[str, dict[str, dict]] = {}
    for mode, reads in arms.items():
        histogram: dict[float, int] = {}
        for _, _, boxes, _ in reads:
            for box in boxes:
                histogram[box["confidence"]] = histogram.get(box["confidence"], 0) + 1
                every[box["confidence"]] = every.get(box["confidence"], 0) + 1
        spread = ", ".join(f"{value:.2f}: {histogram[value]}" for value in sorted(histogram))
        print(f"  {mode:7} {sum(histogram.values()):>5} {len(histogram):>8} {spread}")
        terms[mode] = {
            photo_id: confidence_terms(base, boxes, weights)
            for photo_id, base, boxes, _ in reads
            if read_at(base, boxes, 0.0) is not None
        }
    spread = ", ".join(f"{value:.2f}: {every[value]}" for value in sorted(every))
    print(f"  {'all':7} {sum(every.values()):>5} {len(every):>8} {spread}")
    print("  every box under 1.00:")
    for mode, reads in arms.items():
        for photo_id, _, boxes, _ in reads:
            for index, box in enumerate(boxes):
                if box["confidence"] < 1.0:
                    averaged = photo_id in terms[mode] and index in terms[mode][photo_id]["used"]
                    print(
                        f"    {mode:7} {photo_id:13} {box['text']!r:12} {box['confidence']:.2f} at "
                        f"{box['x']:.0f},{box['y']:.0f}, {'averaged' if averaged else 'not averaged'}"
                    )

    print("\n2. Each shot's parse_confidence: the coverage terms less any penalty, and the OCR term, W_OCR x the mean")
    print(f"  {'mode':7} {'id':13} {'boxes':>5} {'averaged':>8} {'conf':>6} {'coverage':>8} {'OCR':>6} {'mean':>7}  not averaged")
    for mode, reads in arms.items():
        for photo_id, _, boxes, _ in reads:
            parts = terms[mode].get(photo_id)
            if parts is None:
                print(f"  {mode:7} {photo_id:13} {len(boxes):>5} {'-':>8} failed read: nothing to average")
                continue
            left_out = [box["text"] for index, box in enumerate(boxes) if index not in parts["used"]]
            print(
                f"  {mode:7} {photo_id:13} {len(boxes):>5} {len(parts['used']):>8} {parts['parse_confidence']:>6.3f} "
                f"{parts['coverage']:>8.3f} {parts['ocr_term']:>6.3f} {parts['mean']:>7.4f}  {left_out}"
            )
    for name, modes in (("phone modes", PHONE_MODES), ("all modes", tuple(arms))):
        counted: dict[float, int] = {}
        for mode in modes:
            for parts in terms[mode].values():
                counted[parts["ocr_term"]] = counted.get(parts["ocr_term"], 0) + 1
        print(f"  the OCR term, {name}: " + ", ".join(f"{value:.3f} on {counted[value]}" for value in sorted(counted)))
    print("  a box's confidence moved no value and no physics flag on any shot (`confidence_terms` stops if one does)")

    at_bars = record["min_confidence"]
    print(f"\n3. Why each flagged shot is flagged, at {at_bars}")
    table: dict[float, dict[str, dict]] = {rung: {} for rung in WHAT_IF}
    for mode, reads in arms.items():
        for rung in WHAT_IF:
            table[rung][mode] = what_if_arm(reads, rung)
    for mode in arms:
        row = table[at_bars][mode]
        for photo_id in row["flagged"]:
            print(f"  {mode:7} {photo_id:13} {row['causes'][photo_id]}: {held_set['arms'][mode]['shots'][photo_id]['warnings']}")
        if not row["flagged"]:
            print(f"  {mode:7} none flagged")
    causes = [cause for mode in arms for cause in table[at_bars][mode]["causes"].values()]
    print(f"  from confidence alone: {sum(cause == 'confidence alone' for cause in causes)} of {len(causes)}")

    print("\n4. The what-if: each threshold re-parsed, and every cell classified again")
    print(f"  {'min_confidence':14} {'mode':7} {'read':>4} {'wrong':>5} {'caught':>6} {'T4':4} flagged")
    for rung in WHAT_IF:
        for mode in arms:
            row = table[rung][mode]
            within = "ok" if len(row["flagged"]) <= ceiling else "OVER"
            flagged = [f"{photo_id} ({row['causes'][photo_id]})" for photo_id in row["flagged"]]
            print(f"  {rung:<14} {mode:7} {row['read']:>4} {len(row['wrong']):>5} {len(row['caught']):>6} {within:4} {flagged}")
    differing = sorted(
        {(mode, *cell[:2]) for rung in WHAT_IF for mode in arms for name in ("wrong", "caught") for cell in table[rung][mode][name]}
    )
    if not differing:
        print("  no cell differs from the truth, on any mode, at any threshold")
    for mode, photo_id, field in differing:
        states = {
            rung: "wrong" if any(cell[:2] == (photo_id, field) for cell in table[rung][mode]["wrong"]) else "caught"
            for rung in WHAT_IF
        }
        print(f"  {mode} {photo_id} {field} differs: {states}")

    # A wrong cell's own threshold is what this phase asks for, and no arm has one, so the one route to a
    # wrong cell that has been seen is shown beside it. P13 found it, on a mutation. It is labelled as one.
    vector = json.loads(GRID_VECTOR.read_text(encoding="utf-8"))["input"]
    garbled = retext(vector["boxes"], "SMASHFACTOR", "XXXXXXXX")
    route = [("the grid, `SMASHFACTOR` garbled", vector, garbled, truth("IMG_2889"))]
    print("\n  A mutation, not a photo: P13's route to a wrong cell, on the committed grid vector's boxes")
    print(f"    parse_confidence {read_at(vector, garbled, 0.0)['provenance']['parse_confidence']}")
    for rung in (*WHAT_IF, 0.95):
        row = what_if_arm(route, rung)
        print(f"    at {rung:<5} {len(row['wrong'])} wrong and {len(row['caught'])} caught, flagged: {bool(row['flagged'])}")
    tiles = len(fields())
    print(f"  Arithmetic, not a measurement: labels a shot must lose to flag, every box at 1.00, of {tiles}:")
    print("    " + ", ".join(f"at {rung}: {tiles_to_flag(weights, tiles, rung)}" for rung in WHAT_IF))
    floor = weights["W_LABELS"] + weights["W_VALUES"]
    print(f"  Arithmetic, not a measurement: a shot with every label and value is {floor} with every box at 0, so at or")
    print(f"    under {floor} no confidence Vision reports can flag it. One box at 0.30 of 34 costs {weights['W_OCR'] * 0.7 / 34:.3f}.")

    print(f"\nThe recommendation, by §P16's rule{label}")
    chosen = recommend(table, ceiling)
    phone_differs = any(table[rung][mode][name] for rung in WHAT_IF for mode in PHONE_MODES for name in ("wrong", "caught"))
    for rung in RUNGS:
        print(f"  {rung}: {'holds' if rung_holds(table[rung], ceiling) else 'does not hold'}")
    if not phone_differs:
        why = "no cell on the phone modes differs from the truth at any threshold, so the rule had nothing to catch"
    elif rung_holds(table[chosen], ceiling):
        why = "the lowest that flags every shot holding a wrong cell, within T4"
    else:
        why = "none does, so the rule's default"
    print(f"  min_confidence {chosen}: {why}")
    print("  It changes no code and no vector: `ScreenInput.min_confidence` is the caller's. `W_OCR` is M38's to name.")
    return 0


# --- verdict (P17) ------------------------------------------------------------------------------------


LOG = SPIKE / "log.md"

# Call 11's item 4: an accepted rule brings a file of this name, and a withdrawn one takes it away by hand.
# On disk rather than a key in `results/`, because a rule changes the parser and no measurement says so.
PARSER_RULE_VECTORS = "spec/vectors/screen/hand/vision-*.json"

# The decision of 2026-11-04's P17 row: "whichever way it reads", beside any label `measure` found on data.
NOT_YET = "This is not yet the answer on the photos as captured."

# Call 9's second row, in `thresholds.md`'s words.
MEANING = {
    "GO": "stills pass on a phone path",
    "CONDITIONAL": "Vision reads the screen; M38 must also ship a rectifier",
    "NO-GO": "the app waits on the user",
}


def judged_again(capture_set: str, held_set: dict, limits: dict[str, int]) -> dict[str, dict]:
    """T1 to T4 for one set's three arms, worked from each cell's own value and not from what the file concluded.

    `failures` judges again from the `cell` strings. This goes one step further back, to the value, the
    truth table and the flag, because a milestone's answer rests on it: a `cell`, a `summary`, a `bars`
    or a `pass` that disagrees stops it, as does a limit that is not `thresholds.md`'s.
    """
    if held_set["limits"] != limits:
        raise SystemExit(f"verdict: STOP: {capture_set}: the limits in results/ are not {THRESHOLDS.name}'s (rule 2)")
    arms = held_set["arms"]
    if set(arms) != set(MODES):
        # `measure --mode` can leave one arm in the file, and "passes on none" with an arm unmeasured
        # would be a NO-GO nobody measured, the same fault P10 warned of on the clips.
        raise SystemExit(f"verdict: STOP: {capture_set} holds the arms {sorted(arms)}, and call 9 reads {list(MODES)}")
    judged = {}
    for mode in MODES:
        arm = arms[mode]
        if len(arm["shots"]) != limits["photos"]:
            raise SystemExit(f"verdict: STOP: {capture_set} {mode}: {len(arm['shots'])} shots, and the bars are for {limits['photos']}")
        shots = {}
        for photo_id, shot in arm["shots"].items():
            parsed = None
            if not shot["failed_read"]:
                # A key `classify` asks for and the file does not hold reads as withheld, and then the
                # comparison below stops on it, rather than a KeyError standing in for a sentence.
                values = {field: shot["cells"].get(field, {}).get("value") for field in fields()}
                parsed = {**values, "provenance": {"needs_review": shot["flagged"]}}
            shots[photo_id] = classify(parsed, truth(photo_id))
            if shots[photo_id]["cells"] != shot["cells"] or shots[photo_id]["flagged"] != shot["flagged"]:
                raise SystemExit(f"verdict: STOP: {capture_set} {mode} {photo_id}: its cells do not classify as results/ says")
        summary = summarize(list(shots.values()))
        bars = judge(summary, limits)
        if summary != arm["summary"] or bars != arm["bars"] or all(bars.values()) != arm["pass"]:
            raise SystemExit(f"verdict: STOP: {capture_set} {mode}: its cells do not judge as results/ says")
        judged[mode] = {"summary": summary, "bars": bars, "pass": all(bars.values()), "shots": shots}
    return judged


def stills_verdict(passed: dict[str, bool]) -> str:
    """Call 9's table. `lab` counts only where neither phone path does: that is what CONDITIONAL is."""
    if any(passed[mode] for mode in PHONE_MODES):
        return "GO"
    return "CONDITIONAL" if passed["lab"] else "NO-GO"


def clips_verdict(passed: dict[str, bool] | None) -> str:
    """Call 9's clip column, with `None` for a set not measured.

    **MISS** tells M38 to ship the still import first, so it must not be what an absent `fs-clip` falls
    through to: that would state a result nobody measured (P10).
    """
    if passed is None:
        return "not measured"
    if any(passed[mode] for mode in PHONE_MODES):
        return "PASS"
    return "LAB-ONLY" if passed["lab"] else "MISS"


def reckon(config: str, record: dict, all_bars: dict[str, dict[str, int]], parser_rules: list[str]) -> dict:
    """The verdict and its labels from one config's record, with nothing printed and no file read.

    Only the two sets call 9 has a column for are opened. HD Golf has no bar and no verdict (decision 3),
    so a `sets.hd` key could not move this if it were there (P11).
    """
    stills = record["sets"].get("fs-still")
    if stills is None:
        raise SystemExit(f"verdict: STOP: {config} holds no fs-still set, and call 9 has nothing to read")
    judged = {"fs-still": judged_again("fs-still", stills, all_bars["fs-still"])}
    clips = record["sets"].get("fs-clip")
    if clips is not None:
        judged["fs-clip"] = judged_again("fs-clip", clips, all_bars["fs-clip"])

    def passed(capture_set: str) -> dict[str, bool]:
        return {mode: judged[capture_set][mode]["pass"] for mode in MODES}

    tuning = "tuned, parser" if parser_rules else None if config == "C0" else "tuned"
    verdict_word = stills_verdict(passed("fs-still"))
    clip_word = clips_verdict(passed("fs-clip") if clips is not None else None)
    label = stills["label"]
    clip_label = clips["label"] if clips is not None else None
    # The label comes from `results/`, where `measure` set it from the data (P7), and no argument can drop it.
    said = ", ".join(part for part in (verdict_word, tuning, label) if part)
    clip_said = ", ".join(part for part in (clip_word, clip_label) if part)
    return {
        "judged": judged,
        "verdict": verdict_word,
        "clips": clip_word,
        "tuning": tuning,
        "label": label,
        "line": f"**Verdict: {said}. Clips: {clip_said}.**",
    }


def bar_rows(judged: dict[str, dict], limits: dict[str, int]) -> list[str]:
    """Call 9's bars for one set as Markdown, which `log.md` must hold verbatim."""

    def marked(text: str, held_up: bool) -> str:
        return text if held_up else f"**{text}, miss**"

    rows = [
        f"| mode | T1 wrong (at most {limits['T1']}) | T2 read (at least {limits['T2']}) | "
        f"T3 lowest field (at least {limits['T3']} of {limits['photos']}) | T4 flagged (at most {limits['T4']}) | arm |",
        "|---|---|---|---|---|---|",
    ]
    for mode in MODES:
        summary, bars = judged[mode]["summary"], judged[mode]["bars"]
        lowest = min(summary["per_field"].values())
        names = ", ".join(f"`{field}`" for field, count in summary["per_field"].items() if count == lowest)
        rows.append(
            f"| `{mode}` | {marked(str(summary['wrong']), bars['T1'])} | {marked(str(summary['read']), bars['T2'])} | "
            f"{marked(f'{lowest}: {names}', bars['T3'])} | {marked(str(summary['flagged']), bars['T4'])} | "
            f"**{'PASS' if judged[mode]['pass'] else 'MISS'}** |"
        )
    return rows


def photo_rows(judged: dict[str, dict]) -> list[str]:
    """Read per photo, per mode, so that a total in the log can be walked back to the shot that lost it."""
    rows = ["| id | " + " | ".join(f"`{mode}`" for mode in MODES) + " |", "|---|" + "---|" * len(MODES)]
    for photo_id in judged[MODES[0]]["shots"]:
        counts = [
            sum(cell["cell"] == "read" for cell in judged[mode]["shots"][photo_id]["cells"].values()) for mode in MODES
        ]
        rows.append(f"| `{photo_id}` | " + " | ".join(str(count) for count in counts) + " |")
    return rows


def log_disagreement(line: str, rows: list[str], text: str) -> list[str]:
    """What `log.md` does not state as this reckoning does. Whole lines, so a near miss is a miss.

    A second `**Verdict:` line is a disagreement too, since a log still holding an old verdict beside
    the new one states both.
    """
    lines = text.splitlines()
    stated = [held for held in lines if held.startswith("**Verdict:")]
    wrong = [] if stated == [line] else [f"its verdict lines are {stated}, and this is {line!r}"]
    return wrong + [f"it has no line {row!r}" for row in rows if row not in lines]


def verdict(_args: argparse.Namespace) -> int:
    """Call 9's table worked again from `results/`, printed, and held to what `log.md` states.

    Runs no Vision and writes nothing: a re-read would rewrite `repeat`, P9's record that two runs
    matched. A NO-GO exits 0, because a miss is a result, as in `measure`. It exits 1 where the answer
    would rest on something other than the measurement, or where the log says something else.
    """
    paths = sorted(RESULTS.glob("*.json"))
    if len(paths) != 1:
        # Call 10's choice among configs is not written, because P12's ladder never ran, and this is
        # not the place to guess at it untested. The row that measures a second config hits this at once.
        print(f"verdict: STOP: {len(paths)} configs in {shown(RESULTS)}, and call 10's choice among several is not written")
        return 1
    results_file = paths[0]
    record = json.loads(results_file.read_text(encoding="utf-8"))
    config = record["config"]
    if results_file.stem != config:
        print(f"verdict: STOP: {shown(results_file)} says it is {config}")
        return 1
    if record["thresholds_sha256"] != digest(THRESHOLDS):
        print(f"verdict: STOP: {THRESHOLDS.name} is not the file {shown(results_file)} was judged against (rule 2)")
        return 1
    if record["min_confidence"] != MIN_CONFIDENCE:
        print(f"verdict: STOP: {shown(results_file)} was measured at {record['min_confidence']}, and call 9 says {MIN_CONFIDENCE}")
        return 1
    parser_rules = sorted(shown(path) for path in ROOT.glob(PARSER_RULE_VECTORS))
    all_bars = read_bars()
    report = reckon(config, record, all_bars, parser_rules)

    # The committed boxes are the measurement's evidence once `frames/` is gone. `fs-clip` has no layout
    # here to hold: a clip's id is its still's, so `boxes/` cannot tell them apart yet (P16).
    for mode, arm in record["sets"]["fs-still"]["arms"].items():
        for photo_id, shot in arm["shots"].items():
            path = SPIKE / "boxes" / config / mode / f"{photo_id}.json"
            if not path.is_file() or digest(path) != shot["boxes_sha256"]:
                print(f"verdict: STOP: {shown(path)} is not the boxes {shown(results_file)} recorded")
                return 1

    rows = []
    print(f"verdict, from {shown(results_file)}, each cell classified and each bar judged again:\n")
    print(report["line"])
    if report["label"]:
        print(NOT_YET)
    print(f"\n{report['verdict']}: {MEANING[report['verdict']]}.")
    print(f"final config {config}: the only one measured, so {report['tuning'] or 'neither tuned label'}")
    for capture_set in ("fs-still", "fs-clip"):
        if capture_set not in report["judged"]:
            print(f"\n{capture_set}: not measured")
            continue
        judged = report["judged"][capture_set]
        arms = record["sets"][capture_set]["arms"]
        hosts = sorted({f"macOS {arm['os']}, Vision revision {arm['revision']}" for arm in arms.values()})
        label = record["sets"][capture_set]["label"]
        print(f"\n{capture_set}" + (f" ({label})" if label else "") + f", {config}, {'; '.join(hosts)}:\n")
        table = bar_rows(judged, all_bars[capture_set]) + [""] + photo_rows(judged)
        print("\n".join(table))
        rows += [row for row in table if row]
    print("\nHD Golf: no bar and no verdict (decision 3)")
    print("\non this Mac, of what the decision of 2026-11-04 runs without, by what `check` holds:")
    for group, wanted in outstanding():
        present = sum(held(path, sha256) == "ok" for path, sha256 in wanted)
        print(f"  {group}: {present} of {len(wanted)}")

    if not LOG.is_file():
        print(f"\nverdict: {LOG.name} is not written, so nothing here is held to it")
        return 1
    disagreed = log_disagreement(report["line"], rows, LOG.read_text(encoding="utf-8"))
    if disagreed:
        print(f"\nverdict: {LOG.name} says otherwise:")
        for item in disagreed:
            print(f"  {item}")
        return 1
    print(f"\n{LOG.name} states the same verdict and the same {len(rows)} table lines")
    return 0


# --- control ----------------------------------------------------------------------------------------


def retext(boxes: list[dict], old: str, new: str) -> list[dict]:
    return [{**box, "text": new} if box["text"] == old else box for box in boxes]


def without_box(boxes: list[dict], text: str) -> list[dict]:
    return [box for box in boxes if box["text"] != text]


def without_rgb(boxes: list[dict], text: str) -> list[dict]:
    return [{k: v for k, v in box.items() if k != "rgb"} if box["text"] == text else box for box in boxes]


def with_rgb(boxes: list[dict], text: str, rgb: list[float]) -> list[dict]:
    return [{**box, "rgb": rgb} if box["text"] == text else box for box in boxes]


def named_once(boxes: list[dict], text: str) -> list[dict]:
    """The box a mutation targets exists exactly once, or the mutation would quietly be no mutation."""
    if sum(box["text"] == text for box in boxes) != 1:
        raise SystemExit(f"control: STOP: {text!r} is not one box in {GRID_VECTOR.name}")
    return boxes


def synthetic(photos: int, changes: tuple = (), flagged: int = 0) -> list[dict]:
    """A set of all-read shots with `changes` applied as (photo, field index, cell), for the bars' control."""
    names = fields()
    shots = [
        {"flagged": index < flagged, "cells": {name: {"cell": "read"} for name in names}}
        for index in range(photos)
    ]
    for photo, field, cell in changes:
        shots[photo]["cells"][names[field]]["cell"] = cell
    return shots


def verdict_controls(seen: Callable[[str, bool, str], None]) -> None:
    """Control 8: call 9's table, on arms built the way `run_arm` builds them, with no Vision and no photo.

    Every row of the table is walked, since the real record reaches one of them. The refusals are here
    because `reckon` works the answer again precisely so that a results file cannot talk it out of it.
    """
    print("control 8: the verdict, every row of call 9's table, and what must stop it")
    limits = read_bars()
    still_ids = [row[0] for row in FS_STILLS]
    clip_ids = [name.split(".")[0] for name, _sha256 in FS_CLIPS]

    def arm(
        capture_set: str, ok: bool = True, changed: dict | None = None, flagged: tuple = (), ids: list[str] | None = None
    ) -> dict:
        """All read, or `ok=False` for one failed read, which is T2's miss on either set and nothing else's."""
        shots = {}
        for index, photo_id in enumerate(ids or (still_ids if capture_set == "fs-still" else clip_ids)):
            want = truth(photo_id)
            parsed = None
            if ok or index != 0:
                parsed = {**want, **(changed or {}).get(photo_id, {}), "provenance": {"needs_review": photo_id in flagged}}
            shots[photo_id] = {**classify(parsed, want), "failed_read": parsed is None}
        summary = summarize(list(shots.values()))
        bars = judge(summary, limits[capture_set])
        return {"summary": summary, "bars": bars, "pass": all(bars.values()), "shots": shots}

    def built(capture_set: str, passes: tuple[bool, bool, bool], label: str | None = None) -> dict:
        arms = {mode: arm(capture_set, ok) for mode, ok in zip(MODES, passes, strict=True)}
        return {"label": label, "limits": limits[capture_set], "arms": arms}

    def record(stills: dict, clips: dict | None = None) -> dict:
        return {"sets": {"fs-still": stills} | ({} if clips is None else {"fs-clip": clips})}

    every = (True, True, True)
    for passes, want_still, want_clip in (
        ((True, True, True), "GO", "PASS"),
        ((True, True, False), "GO", "PASS"),
        ((True, False, True), "GO", "PASS"),
        ((True, False, False), "GO", "PASS"),
        ((False, True, True), "GO", "PASS"),
        ((False, True, False), "GO", "PASS"),
        ((False, False, True), "CONDITIONAL", "LAB-ONLY"),
        ((False, False, False), "NO-GO", "MISS"),
    ):
        on = dict(zip(MODES, passes, strict=True))
        got = reckon("C0", record(built("fs-still", passes)), limits, [])["verdict"]
        seen(f"stills {on}", got == want_still, got)
        got = reckon("C0", record(built("fs-still", every), built("fs-clip", passes)), limits, [])["clips"]
        seen(f"clips {on}", got == want_clip, got)

    # P10's and P11's two: a set not measured says so, and HD Golf cannot move anything.
    alone = reckon("C0", record(built("fs-still", every)), limits, [])
    seen("no fs-clip key is not measured, and never MISS", alone["clips"] == "not measured", alone["clips"])
    with_hd = record(built("fs-still", every))
    with_hd["sets"]["hd"] = {"arms": "not a set anyone may open"}
    seen("a sets.hd key moves nothing", reckon("C0", with_hd, limits, [])["line"] == alone["line"], alone["line"])

    # A wrong cell is the point of the spike (T1), and a caught one is what review is for.
    photo = still_ids[1]
    wrong = {photo: {"carry_distance": truth(photo)["carry_distance"] + 1}}
    for name, passing_raw, want_word in (
        ("a wrong cell on `raw` alone fails it", arm("fs-still", True, wrong), "NO-GO"),
        ("the same cell on a flagged shot is caught and passes", arm("fs-still", True, wrong, (photo,)), "GO"),
    ):
        arms = {"raw": passing_raw, "vision": arm("fs-still", False), "lab": arm("fs-still", False)}
        held_set = {"label": None, "limits": limits["fs-still"], "arms": arms}
        got = reckon("C0", record(held_set), limits, [])["verdict"]
        seen(name, got == want_word, got)

    # The labels, each from where the plan says it comes.
    pasted = "provisional: pasted copies"
    for name, config, rules, stills, clips, line in (
        ("the first settings on the originals", "C0", [], None, None, "**Verdict: GO. Clips: not measured.**"),
        ("the label comes from the set", "C0", [], pasted, None, f"**Verdict: GO, {pasted}. Clips: not measured.**"),
        ("any other config is tuned", "C1", [], None, None, "**Verdict: GO, tuned. Clips: not measured.**"),
        ("a rule is tuned, parser, at C0 too", "C0", ["a"], None, None, "**Verdict: GO, tuned, parser. Clips: not measured.**"),
        ("both, in order", "C2", ["a"], pasted, None, f"**Verdict: GO, tuned, parser, {pasted}. Clips: not measured.**"),
        ("the clips keep their own label", "C0", [], None, pasted, f"**Verdict: GO. Clips: PASS, {pasted}.**"),
    ):
        clip_set = None if clips is None else built("fs-clip", every, clips)
        got = reckon(config, record(built("fs-still", every, stills), clip_set), limits, rules)["line"]
        seen(f"line, {name}", got == line, got)

    # What must stop it. Each edit is one a hand or a stale run could make, on a set that would otherwise pass.
    base = record(built("fs-still", (False, True, True)))
    raw_shot = next(iter(base["sets"]["fs-still"]["arms"]["raw"]["shots"]))
    other_shot = still_ids[1]

    def shots_of(held: dict) -> dict:
        return held["sets"]["fs-still"]["arms"]["raw"]["shots"]

    def edit_pass(held: dict) -> None:
        held["sets"]["fs-still"]["arms"]["raw"]["pass"] = True

    def edit_bar(held: dict) -> None:
        held["sets"]["fs-still"]["arms"]["raw"]["bars"]["T2"] = True

    def edit_summary(held: dict) -> None:
        held["sets"]["fs-still"]["arms"]["raw"]["summary"]["read"] += 1

    def edit_cell(held: dict) -> None:
        shots_of(held)[raw_shot]["cells"]["carry_distance"]["cell"] = "read"

    def edit_value(held: dict) -> None:
        shots_of(held)[other_shot]["cells"]["carry_distance"]["value"] += 1

    def edit_truth(held: dict) -> None:
        shots_of(held)[other_shot]["cells"]["carry_distance"]["truth"] += 1

    def edit_flag(held: dict) -> None:
        shots_of(held)[raw_shot]["flagged"] = False

    def edit_failed(held: dict) -> None:
        # This direction only. A failed read called a parse, flagged and all withheld, is the same cells
        # by call 8's own words and moves no bar, so there is nothing there for a stop to protect.
        shots_of(held)[other_shot]["failed_read"] = True

    def edit_limit(held: dict) -> None:
        held["sets"]["fs-still"]["limits"]["T2"] -= 12

    def edit_arm(held: dict) -> None:
        del held["sets"]["fs-still"]["arms"]["lab"]

    def edit_shot(held: dict) -> None:
        del shots_of(held)[other_shot]

    def edit_field(held: dict) -> None:
        del shots_of(held)[other_shot]["cells"]["sidespin"]

    def edit_set(held: dict) -> None:
        del held["sets"]["fs-still"]

    def edit_size(held: dict) -> None:
        # True to itself and passing its bars, with six shots against five: only the count can see this,
        # where deleting a shot from an arm is already caught by its summary.
        arms = {mode: arm("fs-clip", ids=still_ids) for mode in MODES}
        held["sets"]["fs-clip"] = {"label": None, "limits": limits["fs-clip"], "arms": arms}

    def stops(edit: Callable[[dict], None]) -> bool:
        held = json.loads(json.dumps(base))
        edit(held)
        try:
            reckon("C0", held, limits, [])
        except SystemExit:
            return True
        return False

    seen("the unedited record does not stop", not stops(lambda _held: None), "it reckons")
    for edit in (
        edit_pass, edit_bar, edit_summary, edit_cell, edit_value, edit_truth, edit_flag, edit_failed, edit_limit,
        edit_arm, edit_shot, edit_field, edit_set, edit_size,
    ):
        seen(f"reckon stops on {edit.__name__}", stops(edit), "stopped" if stops(edit) else "went through")

    # The log is held whole-line, and a stale verdict left beside the new one is a disagreement.
    line, rows = "**Verdict: GO. Clips: not measured.**", ["| `raw` | 0 |"]
    for name, text, agrees in (
        ("the same verdict and rows", f"# t\n\n{line}\n\n| `raw` | 0 |\n", True),
        ("another verdict", "**Verdict: NO-GO. Clips: not measured.**\n\n| `raw` | 0 |\n", False),
        ("a stale verdict beside it", f"{line}\n**Verdict: NO-GO. Clips: MISS.**\n\n| `raw` | 0 |\n", False),
        ("the verdict inside a sentence", f"So {line} it reads.\n\n| `raw` | 0 |\n", False),
        ("a row that differs", f"{line}\n\n| `raw` | 1 |\n", False),
        ("no row", f"{line}\n", False),
    ):
        got = not log_disagreement(line, rows, text)
        seen(f"the log, {name}", got == agrees, f"agrees: {got}")


def control(_args: argparse.Namespace) -> int:
    """Show that the comparator sees each thing it must, before it measures anything.

    No Vision and no photo: committed vector boxes through the binary a measurement uses, and
    synthetic totals. A failed control exits 1. Control 1 or 2 failing on the parser's side would
    contradict `cargo test`, so that is the plan's stop.
    """
    if not GOLF_CORE.is_file():
        print(f"control: STOP: no {GOLF_CORE.relative_to(ROOT)}; the plan's Verify section builds it")
        return 1
    failures: list[str] = []

    def seen(name: str, held_up: bool, detail: str) -> None:
        print(f"  {'ok  ' if held_up else 'FAIL'} {name}: {detail}")
        if not held_up:
            failures.append(name)

    # 1. Every HD Golf vector's committed boxes against its own expected.
    print("control 1: each HD Golf vector's committed boxes, against its own expected (call 14)")
    print(
        f"       {'vector':14} {'boxes':>5} {'read':>4} {'withheld':>8} {'wrong':>5} {'caught':>6} {'extra':>5} "
        "flagged"
    )
    paths = [p for folder in HD_VECTORS for p in sorted((ROOT / folder).glob("*.json"))]
    grand: dict[str, int] = {"read": 0, "withheld": 0, "wrong": 0, "caught": 0, "extra": 0}
    kept: dict[str, tuple[dict, dict]] = {}
    for path in paths:
        vector = json.loads(path.read_text(encoding="utf-8"))
        shot = json.loads(parse_screen(vector["input"]))
        result = classify_vector(path.stem, shot, vector["expected"]["shot"])
        kept[path.stem] = (shot, vector["expected"]["shot"])
        counts = {n: sum(c["cell"] == n for c in result["cells"].values()) for n in grand}
        for name in grand:
            grand[name] += counts[name]
        clean = counts["read"] > 0 and not any(counts[n] for n in ("withheld", "wrong", "caught", "extra"))
        print(
            f"  {'ok  ' if clean else 'FAIL'}  {path.stem:14} {len(vector['input']['boxes']):>5} "
            f"{counts['read']:>4} {counts['withheld']:>8} {counts['wrong']:>5} {counts['caught']:>6} "
            f"{counts['extra']:>5} {result['flagged']}"
        )
        if not clean:
            failures.append(f"control 1: {path.stem}")
    seen(
        "control 1",
        len(paths) == 15 and grand["read"] > 0,
        f"{len(paths)} vectors, {grand['read']} read and no other cell",
    )

    # 1b. The same classifier, shown to tell its cells apart. Without this control 1 proves only that
    # equal things read as equal, and a classifier that said "read" for everything would pass it.
    print("control 1b: call 14's classifier tells its cells apart, on one parsed vector")
    shot, expected = kept["2026-08-23-2"]
    numeric = "club_head_speed"
    empty = "impact_offset_h"

    def one_cell(vector_id: str, source: tuple[dict, dict], key: str, change: dict, flag: bool = False) -> str | None:
        changed = {**source[0], **change, "provenance": {**source[0]["provenance"], "needs_review": flag}}
        return classify_vector(vector_id, changed, source[1])["cells"].get(key, {}).get("cell")

    for name, got, want in (
        ("a value changed", one_cell("2026-08-23-2", (shot, expected), numeric, {numeric: 1.5}), "wrong"),
        ("changed, flagged", one_cell("2026-08-23-2", (shot, expected), numeric, {numeric: 1.5}, True), "caught"),
        ("a value lost", one_cell("2026-08-23-2", (shot, expected), numeric, {numeric: None}), "withheld"),
        ("a value on a null", one_cell("2026-08-23-2", (shot, expected), empty, {empty: 1.5}), "extra"),
        (
            "the declared difference on its own photo",
            one_cell("2026-08-10-1", kept["2026-08-10-1"], "impact_position", {"impact_position": "CENTER"}),
            "read",
        ),
    ):
        seen(f"control 1b, {name}", got == want, f"{got}")
    first_expected = kept["2026-08-10-1"][1]["impact_position"]
    seen("control 1b, the declared vector still holds null", first_expected is None, f"{first_expected!r}")
    elsewhere = one_cell("another-photo", kept["2026-08-10-1"], "impact_position", {"impact_position": "CENTER"})
    seen("control 1b, the same value under any other photo's id", elsewhere == "extra", f"{elsewhere}")
    null_cells = classify_vector("2026-08-23-2", None, expected)
    seen(
        "control 1b, a null shot",
        null_cells["flagged"] and {c["cell"] for c in null_cells["cells"].values()} == {"withheld"},
        f"{len(null_cells['cells'])} withheld, flagged",
    )

    # 2. The one hand-worked Full Swing vector, against the table row it was read from.
    print("control 2: full-swing-grid.json's boxes, against IMG_2889's truth")
    grid = json.loads(GRID_VECTOR.read_text(encoding="utf-8"))["input"]
    want = truth("IMG_2889")
    grid_shot = json.loads(parse_screen(grid))
    baseline = classify(grid_shot, want)
    counts = summarize([baseline])
    seen(
        "control 2",
        counts["read"] == len(want) == 12 and not baseline["flagged"],
        f"{counts['read']} read of {len(want)}, flagged {baseline['flagged']}",
    )

    # 3. Mutations, each one thing the comparator must see, and everything else unmoved. The expected
    # map names every cell that is not `read`, so a mutation that disturbs a second cell fails as well.
    print("control 3: mutations on those boxes, each of which must be seen")
    boxes = grid["boxes"]
    for text in ("9899", "1.08", "R"):
        named_once(boxes, text)
    all_withheld = {name: "withheld" for name in want}
    for name, mutated, moved, flagged in (
        ("`9899` changed to `9099`", retext(boxes, "9899", "9099"), {"backspin": "wrong"}, False),
        ("`1.08` changed to `1.88`", retext(boxes, "1.08", "1.88"), {"smash_factor": "caught"}, True),
        ("the `R` box removed", without_box(boxes, "R"), {"launch_direction": "withheld"}, False),
        ("`rgb` dropped from the `R` box", without_rgb(boxes, "R"), {"launch_direction": "withheld"}, False),
        ("the `R` box reading `L`, still lit", retext(boxes, "R", "L"), {"launch_direction": "wrong"}, False),
        ("`D` and `F` both removed", without_box(without_box(boxes, "D"), "F"), {"sidespin": "withheld"}, False),
        ("the `9899` box removed", without_box(boxes, "9899"), {"backspin": "withheld"}, False),
        ("an empty box list", [], all_withheld, True),
    ):
        shot = json.loads(parse_screen({**grid, "boxes": mutated}))
        result = classify(shot, want)
        got = {f: c["cell"] for f, c in result["cells"].items() if c["cell"] != "read"}
        one = summarize([result])
        detail = "a null shot, " if shot is None else ""
        detail += f"{got if len(got) < 12 else '12 withheld'}, flagged {one['flagged']}"
        seen(name, got == moved and result["flagged"] == flagged, detail)
    unsigned_boxes = without_box(without_box(boxes, "D"), "F")
    unsigned = json.loads(parse_screen({**grid, "boxes": unsigned_boxes}))
    absent = "sidespin" not in unsigned
    rows = lit_rows(boxes, unsigned, lit_constants())
    seen(
        "the lit table on a shot with no sidespin key",
        absent and all(row["decided"] is None for row in rows if row["field"] == "sidespin"),
        f"key absent {absent}, {sum(row['field'] == 'sidespin' for row in rows)} letters, none decided",
    )
    null_shot = classify(json.loads(parse_screen({**grid, "boxes": []})), want)
    seen("a null shot counts as one flagged", summarize([null_shot])["flagged"] == 1, "1 flagged")

    # Rule 10 has no tolerance, and that needs its own proof: every mutation above moves a printed digit.
    ulp = dict(grid_shot)
    ulp["apex_height"] = math.nextafter(grid_shot["apex_height"], math.inf)
    seen(
        "one unit in the last place off (rule 10)",
        classify(ulp, want)["cells"]["apex_height"]["cell"] == "wrong",
        f"{ulp['apex_height']!r} against {want['apex_height']!r} is wrong",
    )

    # 4. The bars. The change counts are typed from the bars by hand and the limits parsed from
    # thresholds.md, so a misread limit fails here as a limit that does not flip where it should.
    print("control 4: the bars, either side of each limit, which flips that bar and only that one")
    bars = read_bars()
    print(f"  parsed from {THRESHOLDS.name}: {bars}")
    for capture_set, photos, spare in (("fs-still", 6, 6), ("fs-clip", 5, 5)):
        limits = bars[capture_set]
        # `spare` is how many cells a set can lose with T2 still met. Spread over that many fields, one
        # each, every field stays at photos - 1, which is T3's floor, so T2 and T3 stay apart.
        table = (
            ("T1", "no wrong", (), 0, True),
            ("T1", "one wrong", ((0, 0, "wrong"),), 0, False),
            ("T2", f"{spare} lost", tuple((0, f, "withheld") for f in range(spare)), 0, True),
            ("T2", f"{spare + 1} lost", tuple((0, f, "withheld") for f in range(spare + 1)), 0, False),
            ("T3", "a field on photos - 1", ((0, 0, "withheld"),), 0, True),
            ("T3", "a field on photos - 2", ((0, 0, "withheld"), (1, 0, "withheld")), 0, False),
            ("T4", "one flagged", (), 1, True),
            ("T4", "two flagged", (), 2, False),
        )
        for bar, name, changes, flagged, passes in table:
            summary = summarize(synthetic(photos, changes, flagged))
            result = judge(summary, limits)
            expected_result = {b: True for b in result}
            expected_result[bar] = passes
            seen(
                f"{capture_set} {bar}, {name}",
                result == expected_result,
                f"read {summary['read']}, lowest field {min(summary['per_field'].values())}, "
                f"wrong {summary['wrong']}, flagged {summary['flagged']}: "
                + " ".join(f"{b} {'pass' if v else 'MISS'}" for b, v in result.items()),
            )
        # Call 9's one sentence on **caught**: not against T1, and not read.
        caught = summarize(synthetic(photos, ((0, 0, "caught"),), 1))
        seen(
            f"{capture_set}, a caught cell",
            caught["wrong"] == 0 and caught["read"] == photos * 12 - 1 and all(judge(caught, limits).values()),
            f"wrong {caught['wrong']}, read {caught['read']}, every bar passes",
        )

    # 5. The lit table says what the binary does. `is_lit` here only explains, and an explanation that
    # disagreed with the parser would mislead P9's margins. With `L` absent an `R` that is lit gives +,
    # and unlit gives -, so the sign of the stored value is the binary's own is_lit.
    print("control 5: the lit table's is_lit against the binary's, at the limits (call 12)")
    constants = lit_constants()
    print(f"  from {PARSER.relative_to(ROOT).as_posix()}: {constants}")
    floor, ratio = constants["LIT_MIN_BRIGHTNESS"], constants["LIT_BLUE_RATIO"]
    edge = ratio * 200.0
    for name, rgb, lit in (
        ("the grid's own R", [238.93, 222.16, 90.45], True),
        ("the grid's own F", [186.54545454545453, 183.2077922077922, 175.62337662337663], False),
        ("blue on the ratio exactly", [200.0, 200.0, edge], True),
        ("blue one step over it", [200.0, 200.0, math.nextafter(edge, math.inf)], False),
        ("the floor exactly", [floor, floor + 10.0, ratio * floor], True),
        ("one step under the floor", [math.nextafter(floor, 0.0), floor + 10.0, 10.0], False),
        ("green the lesser", [floor + 10.0, floor, ratio * floor], True),
    ):
        shot = json.loads(parse_screen({**grid, "boxes": with_rgb(boxes, "R", rgb)}))
        binary = shot["launch_direction"] is not None and shot["launch_direction"] > 0
        here = is_lit(rgb, constants)
        seen(name, here == binary == lit, f"{rgb}: here {here}, the binary {binary}")
    rows = {row["letter"]: row for row in lit_rows(boxes, grid_shot, constants)}
    seen(
        "the grid's letters",
        sorted(rows) == ["D", "F", "R"] and rows["R"]["lit"] and rows["D"]["lit"] and not rows["F"]["lit"]
        and rows["R"]["decided"] == 1.0 and rows["D"]["decided"] == -1.0,
        " ".join(
            f"{letter} {row['ratio']:.3f} margin {row['margin']:+.3f} lit {row['lit']} sign {row['decided']:+.0f}"
            for letter, row in sorted(rows.items())
        ),
    )

    # 6. `whose`, on the same boxes. Its one job is the line between the parser's and the recognizer's,
    # so each case is a miss of known cause and every cell not read must land on the side named.
    print("control 6: call 11's item 1, on mutations whose cause is known")
    merged = [
        {**box, "text": "10.1 R"} if box["text"] == "10.1" else box for box in without_box(boxes, "R")
    ]
    for name, mutated, expected_cells in (
        ("`9899` removed: the number is in no box", without_box(boxes, "9899"), {"backspin": False}),
        ("`9899` misread as `9099`", retext(boxes, "9899", "9099"), {"backspin": False}),
        ("`D` and `F` removed: the magnitude alone", without_box(without_box(boxes, "D"), "F"), {"sidespin": False}),
        ("`rgb` dropped from the `R` box", without_rgb(boxes, "R"), {"launch_direction": False}),
        # Both, and that is right: with its label gone `1.08` spills into the next column, as `142.2` did
        # under `APEX RI` on P9's `lab`, so backspin reads the neighbour's number, and each is still in the boxes.
        (
            "`SMASHFACTOR` garbled, and `1.08` still there",
            retext(boxes, "SMASHFACTOR", "XXXXXXXX"),
            {"smash_factor": True, "backspin": True},
        ),
        ("the `R` merged into its number", merged, {"launch_direction": True}),
        ("an empty box list", [], {name: False for name in want}),
    ):
        shot = json.loads(parse_screen({**grid, "boxes": mutated}))
        raw_fields = {} if shot is None else shot["provenance"]["raw_fields"]
        cells = classify(shot, want)["cells"]
        got = {
            field: whose("IMG_2889", field, mutated, raw_fields)["fixable"]
            for field, cell in cells.items()
            if cell["cell"] != "read"
        }
        seen(name, got == expected_cells, f"{got if len(got) < 12 else '12 cells'}")
    seen(
        "a number seen only elsewhere is present and not in the tile",
        (lambda row: row["fixable"] and row["holding"] and not row["in_tile"])(
            whose("IMG_2889", "backspin", boxes, {"Backspin Rpm": "2284"})
        ),
        "fixable, in_tile False",
    )
    seen(
        "a typographic minus and a thousands comma are the printed number",
        numbers_in("\u22122.0\u00b0") == [-2.0] and numbers_in("9,899") == [9899.0] and numbers_in("R 10.1") == [10.1],
        "\u22122.0 and 9,899 and R 10.1",
    )
    seen(
        "a word holding the letter is not the letter",
        not boxes_with_letter([{"text": "DD", "rgb": [1, 1, 1]}, {"text": "SIDESPIN", "rgb": [1, 1, 1]}], "DF"),
        "`DD` and `SIDESPIN` hold no D",
    )
    seen(
        "the table's spelling, not ShotData's",
        printed("IMG_2889", "apex_height") == 85.2 and printed("IMG_2889", "sidespin") == 4199.0
        and printed("IMG_2889", "club_path") == -2.0,
        "apex 85.2 feet, sidespin 4199 unsigned, path -2.0",
    )

    # 7. `calibrate`'s instruments. Each answers something no output prints, so each is shown on a case
    # whose answer is known: the grid vector's note names the boxes in no cell, and the mutations' causes are set here.
    print("control 7: calibrate's instruments, on the grid vector's boxes and mutations of known cause")
    weights = score_weights()
    used = averaged_boxes(grid, boxes, weights["W_OCR"])
    left_out = sorted(box["text"] for index, box in enumerate(boxes) if index not in used)
    seen(
        "the boxes averaged are the note's, less those in no cell",
        left_out == sorted(["TEAM1", "1", "AARON", "Ev", "DA", "SHOT DATA", "CONTINUE"]),
        f"{len(used)} of {len(boxes)}, leaving out {left_out}",
    )
    grid_terms = confidence_terms(grid, boxes, weights)
    seen(
        "the coverage terms and the OCR term add up",
        grid_terms["coverage"] == 0.7 and grid_terms["parse_confidence"] == round(0.7 + 0.3 * grid_terms["mean"], 3),
        f"coverage {grid_terms['coverage']}, OCR {grid_terms['ocr_term']}, mean {grid_terms['mean']:.5f}",
    )
    # At 0.30, what Vision gives a stray. In a cell it must lower the mean; in no cell it must not.
    low_in_cell = [{**box, "confidence": 0.3} if box["text"] == "9899" else box for box in boxes]
    low_stray = [{**box, "confidence": 0.3} if box["text"] == "CONTINUE" else box for box in boxes]
    in_cell = confidence_terms(grid, low_in_cell, weights)
    stray = confidence_terms(grid, low_stray, weights)
    seen(
        "a low box in a cell moves the OCR term, and a low stray does not",
        in_cell["ocr_term"] < grid_terms["ocr_term"] and stray["ocr_term"] == grid_terms["ocr_term"],
        f"{grid_terms['ocr_term']}, in a cell {in_cell['ocr_term']}, a stray {stray['ocr_term']}",
    )
    smash_broken = retext(boxes, "1.08", "1.80")
    for name, cause_boxes, rung, cause in (
        ("the clean grid", boxes, 0.9, None),
        ("a smash factor that fails the identity", smash_broken, 0.6, "physics"),
        ("every box at 0: 0.70, under 0.9 and over 0.6", all_at(boxes, 0.0), 0.9, "confidence alone"),
        ("the same at 0.6, where nothing fires", all_at(boxes, 0.0), 0.6, None),
        ("no boxes", [], 0.6, "failed read"),
    ):
        got = flag_cause(grid, cause_boxes, rung)
        seen(f"flag_cause, {name}", got == cause, f"{got}")
    spilled = retext(boxes, "SMASHFACTOR", "XXXXXXXX")
    spill_route = [("grid", grid, spilled, want)]
    below, above = what_if_arm(spill_route, 0.9), what_if_arm(spill_route, 0.95)
    seen(
        "the what-if turns a wrong cell into a caught one where the flag starts",
        [cell[1] for cell in below["wrong"]] == ["backspin"] and not below["flagged"] and not below["caught"]
        and [cell[1] for cell in above["caught"]] == ["backspin"] and above["flagged"] == ["grid"] and not above["wrong"],
        f"at 0.9 wrong {[c[1] for c in below['wrong']]}, at 0.95 caught {[c[1] for c in above['caught']]}",
    )
    spilled_at = read_at(grid, all_at(spilled, 1.0), 0.0)["provenance"]["parse_confidence"]
    one_tile = (weights["W_LABELS"] + weights["W_VALUES"]) * 11 / 12 + weights["W_OCR"]
    # Either side of the sum, unrounded, because the record's three decimals round it up past itself: the
    # first spelling of this control asked at 0.942, which is already over 0.94167, and failed for that.
    flagged_at = [
        read_at(grid, all_at(spilled, 1.0), one_tile + nudge)["provenance"]["needs_review"] for nudge in (-1e-6, 1e-6)
    ]
    seen(
        "the lost-label arithmetic is the binary's on the one case measured",
        spilled_at == round(one_tile, 3) and flagged_at == [False, True]
        and tiles_to_flag(weights, 12, one_tile - 1e-6) == 2 and tiles_to_flag(weights, 12, one_tile + 1e-6) == 1,
        f"{spilled_at} with one label lost, every box at 1.00; flagged just under and over {one_tile:.5f}: {flagged_at}",
    )

    def rungs(flagged_by: dict[float, list[str]], wrong_until: float | None, mode: str = "raw") -> dict:
        """A what-if table holding one shot's differing cell, wrong below `wrong_until` and flagged from there."""
        quiet = {"wrong": [], "caught": [], "flagged": []}
        table = {}
        for rung in WHAT_IF:
            wrong = wrong_until is not None and rung < wrong_until
            changed = {
                "wrong": [("a", "carry_distance", 1, 2)] if wrong else [],
                "caught": [("a", "carry_distance", 1, 2)] if wrong_until is not None and not wrong else [],
                "flagged": flagged_by.get(rung, []),
            }
            table[rung] = {name: dict(quiet) for name in PHONE_MODES} | {mode: changed}
        return table

    for name, table, expected in (
        ("no differing cell lands on 0.6", rungs({}, None), 0.6),
        ("wrong at 0.6 and caught at 0.7", rungs({0.7: ["a"], 0.8: ["a"], 0.9: ["a"]}, 0.7), 0.7),
        ("caught only at 0.9", rungs({0.9: ["a"]}, 0.9), 0.9),
        ("on `vision` too, not only `raw`", rungs({0.8: ["a"], 0.9: ["a"]}, 0.8, "vision"), 0.8),
        ("caught only past T4's ceiling falls back to 0.6", rungs({0.9: ["a", "b"]}, 0.9), 0.6),
        ("never caught falls back to 0.6", rungs({}, 1.0), 0.6),
        ("`lab` is no phone mode, so its wrong cell picks nothing", rungs({0.9: ["a"]}, 0.9, "lab"), 0.6),
    ):
        got = recommend(table, 1)
        seen(f"recommend, {name}", got == expected, f"{got}")

    verdict_controls(seen)

    if failures:
        print(f"control: {len(failures)} FAILED")
        for failure in failures:
            print(f"  {failure}")
        return 1
    print("control: all held")
    return 0



def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(required=True)
    commands.add_parser("build-manifest", help="write photos.json").set_defaults(run=build_manifest)
    commands.add_parser("check", help="digests and tools; a mismatch is a stop").set_defaults(run=check)
    commands.add_parser("prepare", help="each capture's raw and lab inputs").set_defaults(run=prepare)
    commands.add_parser("control", help="the comparator sees what it must").set_defaults(run=control)
    measuring = commands.add_parser("measure", help="one config's arms, classified and judged")
    measuring.add_argument("--config", required=True)
    measuring.add_argument("--set", required=True, dest="capture_set")
    measuring.add_argument("--mode", choices=MODES)
    measuring.set_defaults(run=measure)
    reading = commands.add_parser("failures", help="call 11's condition, and whose each unread cell is")
    reading.add_argument("--config", required=True)
    reading.add_argument("--mode", choices=MODES)
    reading.set_defaults(run=failures)
    calibrating = commands.add_parser("calibrate", help="§P16: the confidences, the flags, and each threshold's what-if")
    calibrating.add_argument("--config", required=True)
    calibrating.set_defaults(run=calibrate)
    commands.add_parser("verdict", help="§P17: call 9's table worked again from results/, held to log.md").set_defaults(run=verdict)
    args = parser.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
