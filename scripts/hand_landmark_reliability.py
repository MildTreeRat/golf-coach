"""Are the hand landmarks measurable at address, from the face-on camera? The M14 gate. [M14 P3]

Usage:
    python scripts/hand_landmark_reliability.py
    python scripts/hand_landmark_reliability.py --view down-the-line   # the control

Stdlib and the base install only. Reads the `*.keypoints.json` already stored under
`data/processed/sessions/` — no re-capture, no MediaPipe, no extras. Every stored swing this repo
has ever analyzed already carries landmarks 17-22, and nothing has ever read them.

### The question, and why it is not already answered

`M4_POSE_BAKEOFF.md` §Phase G screened landmark visibility over 584 **down-the-line** GolfDB clips
and put the lead thumb / index / pinky at 0.37 / 0.39 / 0.40, below its 0.60 exclusion floor.
`ROADMAP.md` reads that as grip being unmeasurable with this instrument. The table does not support
the general claim: the **trail** hand cleared the floor comfortably in the same view (0.84-0.85),
and §Phase G itself notes that face-on the lead elbow tracks at 0.93 against 0.46 down-the-line. It
is an anatomy result about one camera — from behind, the lead arm crosses the body and the torso
hides it.

The camera this program actually scores mechanics from has never been screened for hands, and the
window a grip metric would live in is **address**: static golfer, no motion blur, hands at their
most stationary. ADR-017 was careful about the same distinction for the club head — "a fine target
*at address*", and what killed it was blur at swing speed, not visibility at rest.

### The screen

Per landmark, over `measure.address_sample_bounds` and over the whole clip for contrast:

- **tracked-frame fraction** at `measure.MIN_VISIBILITY` — the gate, and the same 0.5 floor
  `tune_z_channel.py` read at, so the fractions sit directly beside §Phase G's;
- mean visibility and p10/p50/p90, which say *how* a landmark fails on the clips where it does.

**The gate is §Phase G's 0.60 tracked-frame fraction**, reused rather than re-chosen so that the two
tables read against each other.

Shoulders and elbows are screened alongside as **controls**, not gated. §Phase G puts shoulders at
1.00 in the *harder* view; if ours do not come back near that, this script is reading the artifacts
or the windows wrong and its hand numbers are not worth anything.

### Two things this cannot decide

**It is an exclusion floor, not a relevance ranking.** §Phase G is explicit and it carries over
unchanged: a pass says a landmark is *visible* at address, never that it is *useful*. Usefulness is
settled by fitting, the way §Phase E settled `z` — a channel passed its screen and lost the fit.

**The corpus is fifteen face-on swings from one golfer**, against §Phase G's 584 clips from a
broadcast population. That asymmetry is why the per-clip column sits beside the pooled one: a
landmark tracked perfectly in nine swings and absent in six pools to 0.60 and clears the floor while
being unusable on 40% of swings. At 584 clips that distinction is cosmetic; at 15 it is the answer.

Prints a table and changes nothing. The verdict moves into `docs/M14_HAND_LANDMARKS.md` by hand.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path
from typing import NamedTuple

from golf_coach.analysis.measure import MIN_VISIBILITY, address_sample_bounds
from golf_coach.analysis.phases import LEAD_WRIST, TRAIL_WRIST, segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.config import settings
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.storage.keypoints_io import load_keypoints

#: §Phase G's exclusion floor, and `tune_landmarks.py` carries the reasoning: below this a landmark
#: is absent often enough that interpolation would be inventing the swing rather than bridging a
#: blink — the same instinct as `analysis/trajectory.py::MAX_MISSING`.
MIN_TRACKED_FRACTION = 0.60


class View(NamedTuple):
    """A camera: which artifact to read, which stored window to re-apply, which wrist segments."""

    stem: str
    window_key: str
    wrist: PoseLandmark


_VIEWS = {
    "face-on": View("face_on", "face_on_window", LEAD_WRIST),
    # `segment_phases` takes the wrist as a parameter precisely because the lead wrist is tracked in
    # 39% of frames from behind (§Phase F). Passing TRAIL_WRIST is what keeps the down-the-line run
    # a control rather than a second, unrelated failure.
    "down-the-line": View("down_the_line", "down_the_line_window", TRAIL_WRIST),
}

#: Screened as `(label, lead, trail, gated)`. Lead is the left side because `phases.LEAD_WRIST` says
#: so — the repo's standing face-on assumption, and this corpus is one right-handed golfer. Shoulder
#: and elbow are the harness checking itself and are deliberately **not** gated: they are here to
#: fail loudly if the windows or the artifacts are being read wrong, not to be kept or dropped.
_ROWS = (
    ("shoulder", PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER, False),
    ("elbow", PoseLandmark.LEFT_ELBOW, PoseLandmark.RIGHT_ELBOW, False),
    ("wrist", PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST, True),
    ("thumb", PoseLandmark.LEFT_THUMB, PoseLandmark.RIGHT_THUMB, True),
    ("index", PoseLandmark.LEFT_INDEX, PoseLandmark.RIGHT_INDEX, True),
    ("pinky", PoseLandmark.LEFT_PINKY, PoseLandmark.RIGHT_PINKY, True),
)

_SCREENED = tuple(point for _label, lead, trail, _gated in _ROWS for point in (lead, trail))

_VERDICTS = {
    (True, True): "keep both",
    (True, False): "lead only",
    (False, True): "trail only",
    (False, False): "DROP",
}


class Sample:
    """Visibility readings for one landmark over one window, pooled across clips.

    Two views of the same numbers, because at this corpus size they can disagree. `tracked` pools
    every frame and is the number comparable to §Phase G. `per_clip_tracked` keeps one fraction per
    swing, which is what shows a landmark that is fine on most swings and simply gone on the rest.
    """

    def __init__(self) -> None:
        self.visibilities: list[float] = []
        self.per_clip_tracked: list[float] = []

    def add(self, values: list[float]) -> None:
        if not values:
            return
        self.visibilities.extend(values)
        self.per_clip_tracked.append(sum(1 for v in values if v >= MIN_VISIBILITY) / len(values))

    @property
    def tracked(self) -> float:
        if not self.visibilities:
            return float("nan")
        return sum(1 for v in self.visibilities if v >= MIN_VISIBILITY) / len(self.visibilities)

    @property
    def mean(self) -> float:
        return statistics.fmean(self.visibilities) if self.visibilities else float("nan")

    def quantile(self, q: float) -> float:
        if not self.visibilities:
            return float("nan")
        ordered = sorted(self.visibilities)
        return ordered[min(len(ordered) - 1, max(0, round(q * (len(ordered) - 1))))]

    def clips_clearing(self, floor: float) -> int:
        return sum(1 for fraction in self.per_clip_tracked if fraction >= floor)


class Clip(NamedTuple):
    """One stored swing, trimmed and smoothed, with its address window resolved."""

    label: str
    frames: list[FrameKeypoints]
    address: tuple[int, int]


def load_clip(swing_dir: Path, view: View) -> Clip | None:
    """Reproduce the production path over one stored swing, or None if it has no usable phases.

    The stored `*_window` is re-applied before anything else, and it is not optional: the pipeline
    segments phases over the *trimmed* clip, and `address_sample_bounds` anchors to the **end** of
    the ADDRESS segment — so skipping the trim moves the very window this screen exists to measure.
    `check_metric_transfer.py` re-applies it for the same reason, and calls that agreement the
    harness checking itself.
    """
    keypoints_path = swing_dir / f"{view.stem}.keypoints.json"
    analysis_path = swing_dir / "analysis.json"
    if not keypoints_path.exists() or not analysis_path.exists():
        return None

    stored = json.loads(analysis_path.read_text(encoding="utf-8"))
    frames = load_keypoints(keypoints_path).frames
    window = stored.get(view.window_key)
    if window:
        frames = frames[window[0] : window[1]]
    if not frames:
        return None

    smoothed = smooth_keypoints(frames)
    bounds = address_sample_bounds(segment_phases(smoothed, view.wrist))
    if bounds is None:
        return None
    return Clip(f"{swing_dir.parent.name}/{swing_dir.name}", smoothed, bounds)


def collect(
    view: View,
) -> tuple[dict[PoseLandmark, Sample], dict[PoseLandmark, Sample], list[Clip]]:
    """`(address, whole_clip, clips)` — one `Sample` per screened landmark per window."""
    address: dict[PoseLandmark, Sample] = {point: Sample() for point in _SCREENED}
    whole: dict[PoseLandmark, Sample] = {point: Sample() for point in _SCREENED}
    clips: list[Clip] = []

    for keypoints_path in sorted(settings.sessions_dir.rglob(f"{view.stem}.keypoints.json")):
        clip = load_clip(keypoints_path.parent, view)
        if clip is None:
            continue
        clips.append(clip)
        lo, hi = clip.address
        for point in _SCREENED:
            address[point].add([f.landmark(point).visibility for f in clip.frames[lo : hi + 1]])
            whole[point].add([f.landmark(point).visibility for f in clip.frames])

    return address, whole, clips


# ----------------------------------------------------------------------------------- reporting


def _fraction_table(title: str, samples: dict[PoseLandmark, Sample], clips: int) -> None:
    """§Phase G's shape — tracked-frame fraction, lead against trail — plus the per-clip column."""
    print(f"\n{title}")
    print(f"{'landmark':<12s}{'lead (L)':>10s}{'trail (R)':>11s}{'clips>=floor':>16s}   verdict")
    print("-" * 72)
    for label, lead, trail, gated in _ROWS:
        left, right = samples[lead], samples[trail]
        clearing = (
            f"{left.clips_clearing(MIN_TRACKED_FRACTION)}/{clips}"
            f" | {right.clips_clearing(MIN_TRACKED_FRACTION)}/{clips}"
        )
        passes = (left.tracked >= MIN_TRACKED_FRACTION, right.tracked >= MIN_TRACKED_FRACTION)
        verdict = "control" if not gated else _VERDICTS[passes]
        print(f"{label:<12s}{left.tracked:10.2f}{right.tracked:11.2f}{clearing:>16s}   {verdict}")


def _visibility_table(title: str, samples: dict[PoseLandmark, Sample]) -> None:
    """How a landmark fails when it does — a low mean is a different problem from a bimodal one."""
    print(f"\n{title}")
    print(f"{'landmark':<18s}{'mean':>8s}{'p10':>8s}{'p50':>8s}{'p90':>8s}")
    print("-" * 50)
    for label, lead, trail, gated in _ROWS:
        if not gated:
            continue
        for side, point in (("lead", lead), ("trail", trail)):
            sample = samples[point]
            print(
                f"{side + ' ' + label:<18s}{sample.mean:8.2f}{sample.quantile(0.10):8.2f}"
                f"{sample.quantile(0.50):8.2f}{sample.quantile(0.90):8.2f}"
            )


def _verdict(address: dict[PoseLandmark, Sample], clips: int) -> None:
    """The gate, stated in the terms M14 P4/P5 are conditional on.

    The wrists are excluded from the count deliberately: they are already read by `segment_phases`
    and every travel metric, so they cannot fail here without invalidating the shipped panel. What
    M14 turns on is 17-22.
    """
    hands = [
        (f"{side} {label}", samples)
        for label, lead, trail, gated in _ROWS
        if gated and label != "wrist"
        for side, samples in (("lead", address[lead]), ("trail", address[trail]))
    ]
    failed = [name for name, sample in hands if not sample.tracked >= MIN_TRACKED_FRACTION]

    print(f"\n{'=' * 72}")
    print(
        f"GATE - tracked-frame fraction >= {MIN_TRACKED_FRACTION:.2f} at address "
        "(Phase G's floor)"
    )
    print("=" * 72)
    if failed:
        print(
            f"  {len(failed)} of {len(hands)} hand landmarks below the floor: {', '.join(failed)}"
        )
    else:
        print(f"  all {len(hands)} hand landmarks clear the floor at address")
    print(
        "\n  A pass says these landmarks are *visible* from this camera at address. It does not\n"
        "  say they are useful: Phase G's floor is an exclusion floor, not a relevance ranking,\n"
        "  and that question is settled by fitting rather than by this screen.\n"
        f"\n  {clips} clips, one golfer. Nothing is written; the verdict moves into\n"
        "  docs/M14_HAND_LANDMARKS.md by hand."
    )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--view", choices=sorted(_VIEWS), default="face-on")
    args = parser.parse_args(argv)
    view = _VIEWS[args.view]

    address, whole, clips = collect(view)
    if not clips:
        print(
            f"no analyzed {args.view} swings under {settings.sessions_dir} - "
            "run scripts/analyze_swing.py first",
            file=sys.stderr,
        )
        return 1

    address_frames = [hi - lo + 1 for _label, _frames, (lo, hi) in clips]
    print(f"view={args.view}   clips={len(clips)}   visibility floor={MIN_VISIBILITY:.2f}")
    print(
        f"address window: median {statistics.median(address_frames):.0f} frames/clip   "
        f"whole clip: median {statistics.median([len(c.frames) for c in clips]):.0f} frames/clip"
    )

    # Printed output is deliberately ASCII, while the prose above is not: this runs in a Windows
    # console under cp1252, where a printed "§" or em dash comes out as a replacement character and
    # makes a table that is meant to be pasted into a doc unpasteable.
    _fraction_table("ADDRESS WINDOW - tracked-frame fraction", address, len(clips))
    _fraction_table("WHOLE CLIP (contrast) - tracked-frame fraction", whole, len(clips))
    _visibility_table("ADDRESS WINDOW - visibility distribution", address)
    _verdict(address, len(clips))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
