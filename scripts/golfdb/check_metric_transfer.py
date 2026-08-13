"""Does a band cut from broadcast footage mean anything on our phone clips? [M6.5]

Usage:
    python scripts/golfdb/check_metric_transfer.py
    python scripts/golfdb/check_metric_transfer.py --estimator mediapipe:lite

`tune_spatial_metric.py` answers "is this metric signal or jitter?" — a question asked entirely
*inside* the reference corpus. This one asks the question that comes next and has never been asked:
**does the population the band was cut from project the same way ours does?** A band can be
perfectly derived and still be meaningless against our footage, and nothing in the repo would say
so.

## Why `head_hip_offset_impact_norm` needs this and the other five did not

Every shipped checkpoint is a *difference of one landmark across time*: `head_sway`, `hip_sway` and
`hip_shift_at_top` are travel between two instants, `finish_balance` is drift from its own mean,
`tempo_ratio` is a ratio of durations. A camera-geometry bias is very nearly common-mode across two
instants of the same clip, so it cancels in the subtraction.

`head_hip_offset_impact_norm` is the exception. It is an *absolute offset between two different
body parts at one instant* — head centre minus hip centre — and there is no subtraction across
time to cancel anything. Shoulder-width normalisation removes the overall `1/Z` scale, so pure
distance from the camera is handled. What it does **not** remove is **yaw**: a camera that is not
square to the target line converts the head/hip *depth* difference into an apparent horizontal
offset, and at impact the hips have rotated open while the head has not, so the two sit at
genuinely different depths exactly where the metric reads. Broadcast telephoto is near-orthographic
and carefully framed; a hand-held phone in a sim bay is neither.

## The control

Measure the same quantity **at address**, where the body is square to the camera, the hips have not
rotated, and head and hip centres sit at nearly the same depth — so a yaw error has very little
depth difference to convert into offset. Then read the two populations against each other:

- **address offsets agree, impact offsets diverge** -> the divergence is swing mechanics. Our
  golfer really does not stay behind the ball, and the band transfers.
- **address offsets already disagree** -> a static bias is present before the swing starts. The
  band does not transfer, and the metric must not become a checkpoint on this footage.

`impact - address` is reported too: it is the part of the offset the golfer *created during the
swing*, and being a difference across time it is the one form of this quantity that should survive
a camera bias. If the populations agree on the delta but disagree on the absolute, that localises
the problem to the static term precisely.

Nothing is written. Like `tune_spatial_metric.py`, this prints a table and the promotion decision
stays a human call.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from typing import Any

import common
import derive_pose_metrics as derive
import extract_pose

from golf_coach.analysis.measure import (
    address_sample_bounds,
    head_center_points,
    hip_center_points,
    mean_of,
    phase_bounds,
    shoulder_width,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import PhaseSegment, SwingPhase
from golf_coach.storage.keypoints_io import load_keypoints

#: The metric under test. Deliberately not a `--metric` flag over `POSE_MEASUREMENTS`: the control
#: below is specific to *this* quantity's failure mode (a static inter-landmark offset), and a
#: generic flag would imply the same control means something for a travel metric, where it does not.
METRIC = "head_hip_offset_impact_norm"

SESSIONS_DIR = common.REPO_ROOT / "data" / "processed" / "sessions"


def _offset_over(
    keypoints: list[FrameKeypoints], window: tuple[int, int], setup: tuple[int, int]
) -> float | None:
    """Signed (head - hips) dx over `window`, in shoulder widths measured over `setup`.

    The body of `measure_head_hip_offset_impact`, with the window as a parameter so the identical
    arithmetic can read address as well as impact. Sharing the ruler (`setup`) across both is what
    makes the two numbers comparable: a different denominator per instant would put a second
    difference into the comparison this script exists to isolate.
    """
    head = mean_of(head_center_points(keypoints, window[0], window[1]))
    hips = mean_of(hip_center_points(keypoints, window[0], window[1]))
    width = shoulder_width(keypoints, setup[0], setup[1])
    if head is None or hips is None or width is None:
        return None
    return (head[0] - hips[0]) / width


def _both_offsets(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> tuple[float, float] | None:
    """`(address_offset, impact_offset)` for one swing, or None if either is unmeasurable."""
    setup = address_sample_bounds(phases)
    impact = phase_bounds(phases, SwingPhase.IMPACT)
    if setup is None or impact is None:
        return None
    at_address = _offset_over(keypoints, setup, setup)
    at_impact = _offset_over(keypoints, impact, setup)
    if at_address is None or at_impact is None:
        return None
    return at_address, at_impact


# ------------------------------------------------------------------ the reference population


def _corpus_offsets(estimator: str) -> list[tuple[str, float, float]]:
    """`(subject, address, impact)` per face-on GolfDB clip, at the hand-annotated instants.

    Same path as `derive_pose_metrics.py` — labelled events, pixel-aspect corrected, cached
    keypoints — so these numbers are drawn from exactly the population the band is cut from.
    """
    cache = common.KEYPOINTS_DIR / extract_pose.estimator_slug(estimator)
    if not cache.is_dir():
        raise FileNotFoundError(f"no cache at {cache} — run extract_pose.py first")

    out: list[tuple[str, float, float]] = []
    for swing in common.load_swings():
        if swing.view != common.VIEW_FACE_ON:
            continue
        path = cache / f"{swing.source_id}.keypoints.json"
        if not path.exists():
            continue
        frames = load_keypoints(path).frames
        phases = derive._phases_from_events(swing.events, swing.events["start"], len(frames))
        if phases is None:
            continue
        corrected = derive._apply_pixel_aspect(frames, swing.pixel_aspect)
        pair = _both_offsets(corrected, phases)
        if pair is not None:
            out.append((swing.subject, pair[0], pair[1]))
    return out


# ------------------------------------------------------------------------------ our own clips


def _our_offsets() -> list[tuple[str, float, float, float | None]]:
    """`(label, address, impact, stored_impact)` per analyzed bay swing.

    Reproduces the production path rather than approximating it: the stored `face_on_window` is
    re-applied, the frames are smoothed, and phases come from `segment_phases` — so `impact` here
    should reproduce the `head_hip_offset_impact_norm` already in `analysis.json`. That agreement
    is the harness checking *itself*; a mismatch means this script is measuring something the
    pipeline does not.
    """
    out: list[tuple[str, float, float, float | None]] = []
    for analysis_path in sorted(SESSIONS_DIR.rglob("analysis.json")):
        swing_dir = analysis_path.parent
        keypoints_path = swing_dir / "face_on.keypoints.json"
        if not keypoints_path.exists():
            continue

        stored: dict[str, Any] = json.loads(analysis_path.read_text(encoding="utf-8"))
        window = stored.get("face_on_window")
        stored_impact = next(
            (
                m["value"]
                for m in stored.get("swing", {}).get("measurements", [])
                if m["name"] == METRIC
            ),
            None,
        )

        frames = load_keypoints(keypoints_path).frames
        if window:
            frames = frames[window[0] : window[1]]
        smoothed = smooth_keypoints(frames)
        pair = _both_offsets(smoothed, segment_phases(smoothed))
        if pair is None:
            continue

        label = f"{swing_dir.parent.name}/{swing_dir.name}"
        out.append((label, pair[0], pair[1], stored_impact))
    return out


# ----------------------------------------------------------------------------------- reporting


def _summarize(label: str, values: list[float]) -> str:
    ordered = sorted(values)
    return (
        f"  {label:<26} n={len(ordered):>4}  "
        f"p10 {_q(ordered, 0.10):+.3f}  median {statistics.median(ordered):+.3f}  "
        f"p90 {_q(ordered, 0.90):+.3f}"
    )


def _q(ordered: list[float], q: float) -> float:
    from golf_coach.analysis.stats import percentile

    return percentile(ordered, q)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--estimator", default="mediapipe:lite")
    args = parser.parse_args(argv)

    try:
        corpus = _corpus_offsets(args.estimator)
    except FileNotFoundError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    ours = _our_offsets()
    if not corpus or not ours:
        print("error: nothing to compare (corpus or bay swings missing)", file=sys.stderr)
        return 1

    print(f"\n{METRIC} — does the reference band transfer to our footage?")
    print(f"estimator: {args.estimator}\n")

    c_addr = [a for _, a, _ in corpus]
    c_imp = [i for _, _, i in corpus]
    c_delta = [i - a for _, a, i in corpus]
    o_addr = [a for _, a, _, _ in ours]
    o_imp = [i for _, _, i, _ in ours]
    o_delta = [i - a for _, a, i, _ in ours]

    print("GolfDB face-on (broadcast, labelled instants):")
    print(_summarize("at address", c_addr))
    print(_summarize("at impact", c_imp))
    print(_summarize("impact - address", c_delta))

    print("\nOur bay swings (hand-held iPhone, detected instants):")
    print(_summarize("at address", o_addr))
    print(_summarize("at impact", o_imp))
    print(_summarize("impact - address", o_delta))

    print("\nPer swing, and the self-check against what the pipeline stored:")
    for label, at_address, at_impact, stored in ours:
        agree = "" if stored is None else f"  (stored {stored:+.4f})"
        print(
            f"  {label:<26} address {at_address:+.3f}  impact {at_impact:+.3f}  "
            f"delta {at_impact - at_address:+.3f}{agree}"
        )

    # The verdict is the *address* row, which is the whole point of the control: a static
    # disagreement is a camera bias, because at address there is no swing yet to disagree about.
    addr_gap = statistics.median(o_addr) - statistics.median(c_addr)
    delta_gap = statistics.median(o_delta) - statistics.median(c_delta)
    print(
        f"\nmedian address gap  {addr_gap:+.3f} shoulder-widths"
        f"\nmedian delta gap    {delta_gap:+.3f} shoulder-widths"
    )
    print(
        "\nRead it this way: a large address gap with a small delta gap is a static camera bias "
        "and\nthe band does not transfer. A small address gap with a large delta gap is swing "
        "mechanics."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
