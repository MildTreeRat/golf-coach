"""MediaPipe Hands on a golf grip — THROWAWAY. Not part of the `golf_coach` package.

Answers the question in `thresholds.md`, which was committed before any of this ran:

    Does `HandLandmarker` return two correctly-placed, stable 21-point hands on a golf grip,
    at address, from the face-on camera?

Needs the `vision` extra (OpenCV + MediaPipe) and the package installed, because it reads the
stored artifacts through the shipped code rather than re-deriving them.

Subcommands::

    probe.py measure [--view V] [--window W] [--arm A] [--swing S] [--limit N]
    probe.py frames  [--swing S] [--k K]      # annotated crops to eyeball; writes frames/

### Three implementation choices, all made before any number was seen

**The address window comes from `scripts/hand_landmark_reliability.py`, imported.** That script is
M14 P3's screen and it already resolves the stored trim, the smoothing and
`measure.address_sample_bounds` in the order the pipeline does. Re-implementing those fifteen lines
here would let this spike's window drift from the window whose hand-visibility numbers it is meant
to read against — and comparing two tables computed over different frames is the failure mode this
whole probe exists to avoid. Same instinct as the M7 Phase 0 probe reaching into
`analysis/phases.py` privates: measure the shipped thing, not a re-implementation.

**`RunningMode.IMAGE`, not `VIDEO`.** Tracking mode carries a region of interest forward between
frames, which would raise the detection rate by re-using a stale ROI and lower the jitter by
smoothing across frames — it would flatter the model on both axes this probe gates. Independent
frames measure the detector. A deployment can only do better than this number.

**Jitter is compared against *raw* pose landmarks, with the smoothed figure reported beside it.**
`HandLandmarker`'s output is unsmoothed, so raw-against-raw is the like-for-like comparison;
dividing unsmoothed by smoothed would charge the hand model for a filter it never ran through.
`thresholds.md` did not specify, so this is recorded here as the choice it is.
"""

from __future__ import annotations

import argparse
import math
import statistics
import sys
import urllib.request
from pathlib import Path
from typing import NamedTuple

REPO = Path(__file__).resolve().parents[2]
OUT_ROOT = Path(__file__).resolve().parent / "frames"

# `scripts/` is not a package, so the M14 screen is reached by path. Deliberate — see the module
# docstring on why its `load_clip` is imported rather than copied.
sys.path.insert(0, str(REPO / "scripts"))

from hand_landmark_reliability import _VIEWS, load_clip  # noqa: E402

from golf_coach.config import settings  # noqa: E402
from golf_coach.contracts.keypoints import PoseLandmark  # noqa: E402
from golf_coach.storage.keypoints_io import load_keypoints  # noqa: E402

#: Google's hand-landmarker bundle, fetched once into `settings.models_dir` beside the pose ones.
_MODEL_URL = (
    "https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/"
    "float16/latest/hand_landmarker.task"
)

#: The 21-point hand model's own indices. 0 is the wrist, 5 the index MCP (the knuckle a grip
#: metric would live on), 17 the pinky MCP.
HAND_WRIST, HAND_INDEX_MCP, HAND_PINKY_MCP, HAND_MIDDLE_TIP = 0, 5, 17, 12

#: Wrist-to-middle-fingertip on an **open** adult hand is ~0.19 m against a ~0.40 m biacromial
#: width. Printed beside the `span` column as context, and it turned out to settle less than it
#: looked like it would: a hand curled around a grip is genuinely shorter than an open one, so a
#: span well under this figure is equally consistent with a correctly-placed closed hand and with a
#: collapsed blob. Never a gate, and the run recorded it as ambiguous rather than as evidence.
EXPECTED_HAND_SPAN_SW = 0.19 / 0.40

#: Crop sides swept, in shoulder-widths. Pre-registered in `thresholds.md` §"the crop is not
#: cheating" — the gates apply to the best k, and k is reported.
CROP_SWEEP = (1.0, 1.5, 2.0, 3.0)
CROP_PX = 512

#: The gates, copied from `thresholds.md` so the probe prints its own verdict and nobody has to
#: hold three numbers in their head while reading the table.
GATE_DETECTION = 0.60
GATE_PLACEMENT = 0.90
GATE_JITTER = 2.0

#: Pose landmarks this probe reads. Lead is the golfer's left: one right-handed golfer, and
#: `phases.LEAD_WRIST` is the repo's standing face-on assumption.
_SIDES = {
    "lead": (PoseLandmark.LEFT_WRIST, PoseLandmark.LEFT_INDEX, "Left"),
    "trail": (PoseLandmark.RIGHT_WRIST, PoseLandmark.RIGHT_INDEX, "Right"),
}


def ensure_hand_model() -> Path:
    """Local path to `hand_landmarker.task`, downloading once. Mirrors `pose/estimator.py`.

    Copied rather than imported because `ensure_pose_model` formats the pose URL from a variant
    name; the temp-then-rename is the part worth keeping, and it is kept for the reason that
    function gives — an interrupted download otherwise leaves a truncated `.task` that every later
    run accepts and fails to parse.
    """
    settings.models_dir.mkdir(parents=True, exist_ok=True)
    path = settings.models_dir / "hand_landmarker.task"
    if not path.exists():
        tmp = path.with_suffix(".part")
        try:
            urllib.request.urlretrieve(_MODEL_URL, tmp)  # trusted https model bundle
            tmp.replace(path)
        finally:
            tmp.unlink(missing_ok=True)
    return path


class Pose(NamedTuple):
    """One frame's pose geometry in pixels — everything the placement test needs."""

    lead_wrist: tuple[float, float]
    trail_wrist: tuple[float, float]
    lead_index: tuple[float, float]
    trail_index: tuple[float, float]
    shoulder_width: float
    wrist_separation: float

    def wrist(self, side: str) -> tuple[float, float]:
        return self.lead_wrist if side == "lead" else self.trail_wrist

    def index(self, side: str) -> tuple[float, float]:
        return self.lead_index if side == "lead" else self.trail_index


def _px(frame, point: PoseLandmark, width: int, height: int) -> tuple[float, float]:
    lm = frame.landmark(point)
    return (lm.x * width, lm.y * height)


def _dist(a: tuple[float, float], b: tuple[float, float]) -> float:
    return math.hypot(a[0] - b[0], a[1] - b[1])


def _pose_geometry(frame, width: int, height: int) -> Pose | None:
    """Pixel geometry for one frame, or None when the shoulders give no usable ruler."""
    lead_wrist = _px(frame, PoseLandmark.LEFT_WRIST, width, height)
    trail_wrist = _px(frame, PoseLandmark.RIGHT_WRIST, width, height)
    shoulder = _dist(
        _px(frame, PoseLandmark.LEFT_SHOULDER, width, height),
        _px(frame, PoseLandmark.RIGHT_SHOULDER, width, height),
    )
    if shoulder <= 1.0:
        return None
    return Pose(
        lead_wrist=lead_wrist,
        trail_wrist=trail_wrist,
        lead_index=_px(frame, PoseLandmark.LEFT_INDEX, width, height),
        trail_index=_px(frame, PoseLandmark.RIGHT_INDEX, width, height),
        shoulder_width=shoulder,
        wrist_separation=_dist(lead_wrist, trail_wrist),
    )


def _read_range(video: Path, first: int, last: int) -> dict[int, object]:
    """Decode [first, last] sequentially and stop — M1.5's reader, and its reason.

    Sequential rather than `CAP_PROP_POS_FRAMES`: HEVC seeks land on the nearest keyframe and
    silently return a neighbouring frame, and every index here came from a pipeline that counted
    frames from zero.
    """
    import cv2

    cap = cv2.VideoCapture(str(video))
    if not cap.isOpened():
        raise SystemExit(f"could not open {video}")
    out: dict[int, object] = {}
    index = 0
    while index <= last:
        ok, image = cap.read()
        if not ok:
            break
        if index >= first:
            out[index] = image
        index += 1
    cap.release()
    return out


def _video_for(swing_dir: Path, stem: str) -> Path | None:
    """The stored clip for a view. Filenames carry a content hash, so glob rather than construct."""
    matches = sorted(swing_dir.glob(f"{stem}.*.MOV")) + sorted(swing_dir.glob(f"{stem}.*.mov"))
    return matches[0] if matches else None


class Detection(NamedTuple):
    """One returned hand, mapped back into full-frame pixels."""

    points: list[tuple[float, float]]
    label: str
    score: float


def _crop_box(pose: Pose, k: float, width: int, height: int) -> tuple[int, int, int, int]:
    """Square of side `k x shoulder_width` centred on the wrist midpoint, clamped to the frame.

    Clamped rather than padded: a box that runs off the edge at address would mean the golfer's
    hands are at the frame boundary, which is a framing problem this probe should surface as a bad
    crop rather than hide behind black bars.
    """
    cx = (pose.lead_wrist[0] + pose.trail_wrist[0]) / 2
    cy = (pose.lead_wrist[1] + pose.trail_wrist[1]) / 2
    half = max(8.0, k * pose.shoulder_width / 2)
    x0 = int(max(0, min(width - 1, cx - half)))
    y0 = int(max(0, min(height - 1, cy - half)))
    x1 = int(max(x0 + 1, min(width, cx + half)))
    y1 = int(max(y0 + 1, min(height, cy + half)))
    return (x0, y0, x1, y1)


def _detect(landmarker, image, box: tuple[int, int, int, int] | None) -> list[Detection]:
    """Run the model on the full frame or on a crop, and return full-frame pixel landmarks."""
    import cv2
    import mediapipe as mp

    height, width = image.shape[:2]
    if box is None:
        x0, y0, x1, y1 = 0, 0, width, height
        patch = image
    else:
        x0, y0, x1, y1 = box
        patch = cv2.resize(image[y0:y1, x0:x1], (CROP_PX, CROP_PX), interpolation=cv2.INTER_AREA)

    rgb = cv2.cvtColor(patch, cv2.COLOR_BGR2RGB)
    result = landmarker.detect(mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb))

    span_x, span_y = x1 - x0, y1 - y0
    out: list[Detection] = []
    for points, handedness in zip(result.hand_landmarks, result.handedness, strict=False):
        out.append(
            Detection(
                points=[(x0 + lm.x * span_x, y0 + lm.y * span_y) for lm in points],
                label=handedness[0].category_name,
                score=handedness[0].score,
            )
        )
    return out


def _assign(detections: list[Detection], pose: Pose) -> dict[str, Detection | None]:
    """Place each detection on a pose wrist, or nowhere.

    `thresholds.md`: placed when the detection's wrist is nearer one pose wrist than the other **by
    half the pose wrist separation**. The margin is what makes this a test rather than a tautology —
    without it every detection is trivially "nearest" to something, including one sitting between
    the two hands or out on the forearm.
    """
    placed: dict[str, Detection | None] = {"lead": None, "trail": None}
    for det in detections:
        wrist = det.points[HAND_WRIST]
        d_lead = _dist(wrist, pose.lead_wrist)
        d_trail = _dist(wrist, pose.trail_wrist)
        side = "lead" if d_lead < d_trail else "trail"
        margin = abs(d_lead - d_trail)
        if margin < 0.5 * pose.wrist_separation:
            continue
        # Two detections claiming the same wrist: keep the nearer one, so the "distinct" count
        # below reports a collapse instead of quietly crediting the second hand.
        current = placed[side]
        if current is None or _dist(wrist, pose.wrist(side)) < _dist(
            current.points[HAND_WRIST], pose.wrist(side)
        ):
            placed[side] = det
    return placed


class Tally:
    """Counters for one (arm, k) configuration, pooled across clips."""

    def __init__(self) -> None:
        self.frames = 0
        self.any_hand = 0
        self.two_hands = 0
        self.detections = 0
        self.placed = 0
        self.distinct = 0
        self.side_frames = {"lead": 0, "trail": 0}
        self.label_agrees = {"lead": 0, "trail": 0}
        self.steps: dict[str, list[float]] = {"lead": [], "trail": []}
        self.pose_steps_raw: dict[str, list[float]] = {"lead": [], "trail": []}
        self.pose_steps_smooth: dict[str, list[float]] = {"lead": [], "trail": []}
        self.offsets: dict[str, list[float]] = {"lead": [], "trail": []}
        # Added after the frame dump showed what the placement gate cannot see: that gate tests
        # landmark 0 only, so a detection whose remaining twenty points are heaped on the shaft
        # still scores "placed". These two say whether the *interior* of the hand was laid out —
        # the knuckle against the pose model's own knuckle, and the wrist-to-fingertip span
        # against the ~0.47 shoulder-widths a real hand measures. Recorded, never gated.
        self.knuckle_offsets: dict[str, list[float]] = {"lead": [], "trail": []}
        self.spans: dict[str, list[float]] = {"lead": [], "trail": []}

    def detection_rate(self) -> float:
        return self.two_hands / self.frames if self.frames else float("nan")

    def placement_rate(self) -> float:
        return self.placed / self.detections if self.detections else float("nan")

    def distinct_rate(self) -> float:
        return self.distinct / self.two_hands if self.two_hands else float("nan")

    def side_rate(self, side: str) -> float:
        return self.side_frames[side] / self.frames if self.frames else float("nan")

    def jitter(self, side: str, pose_steps: dict[str, list[float]]) -> float:
        mine, theirs = self.steps[side], pose_steps[side]
        if not mine or not theirs:
            return float("nan")
        base = statistics.median(theirs)
        return statistics.median(mine) / base if base > 0 else float("inf")


def _step_medians(values: list[tuple[int, tuple[float, float]]], scale: float) -> list[float]:
    """Frame-to-frame displacement over *consecutive* frames only, in shoulder-widths.

    Consecutive matters: a gap means the model lost the hand and came back, and the jump across
    that gap is the golfer moving, not the model shaking.
    """
    out: list[float] = []
    for (i, a), (j, b) in zip(values, values[1:], strict=False):
        if j == i + 1:
            out.append(_dist(a, b) / scale)
    return out


def _windows(clip, stored, window: str) -> tuple[int, int]:
    """Index bounds within the trimmed clip for the named window."""
    if window == "address":
        return clip.address
    if window == "clip":
        return (0, len(clip.frames) - 1)
    raise SystemExit(f"unknown window {window!r}")


def cmd_measure(args: argparse.Namespace) -> int:
    from mediapipe.tasks import python as mp_python
    from mediapipe.tasks.python import vision

    view = _VIEWS[args.view]
    model = ensure_hand_model()
    options = vision.HandLandmarkerOptions(
        base_options=mp_python.BaseOptions(model_asset_path=str(model)),
        running_mode=vision.RunningMode.IMAGE,
        num_hands=2,
        # Post-hoc diagnostic only (`--min-confidence`), and it cannot move a verdict: the gates in
        # `thresholds.md` were run at the model's defaults. Lowering the palm-detector threshold
        # separates "the second hand was never found" from "the second hand was found and
        # suppressed as an overlapping duplicate", which is the difference between a model that is
        # blind to a grip and one that is deduplicating it.
        min_hand_detection_confidence=args.min_confidence,
        min_hand_presence_confidence=args.min_confidence,
    )

    arms: list[tuple[str, float | None]] = []
    if args.arm in ("full", "both"):
        arms.append(("full", None))
    if args.arm in ("crop", "both"):
        arms.extend(("crop", k) for k in ([args.k] if args.k else list(CROP_SWEEP)))

    tallies = {arm: Tally() for arm in arms}
    clips = 0

    with vision.HandLandmarker.create_from_options(options) as landmarker:
        for keypoints_path in sorted(settings.sessions_dir.rglob(f"{view.stem}.keypoints.json")):
            swing_dir = keypoints_path.parent
            if args.swing and args.swing not in str(swing_dir):
                continue
            clip = load_clip(swing_dir, view)
            video = _video_for(swing_dir, view.stem)
            if clip is None or video is None:
                continue

            lo, hi = _windows(clip, None, args.window)
            selected = clip.frames[lo : hi + 1]
            if not selected:
                continue
            # Raw keypoints for the jitter denominator, sliced to the same frames by frame_index
            # rather than by position — the trim and the window are two different offsets and
            # lining them up by arithmetic is how an off-by-one gets into a ratio.
            raw_by_index = {f.frame_index: f for f in load_keypoints(keypoints_path).frames}

            first, last = selected[0].frame_index, selected[-1].frame_index
            images = _read_range(video, first, last)
            if not images:
                continue
            clips += 1
            print(f"  {clip.label}: frames {first}-{last} ({len(selected)})", flush=True)

            tracks: dict[tuple[str, float | None], dict[str, list]] = {
                arm: {"lead": [], "trail": []} for arm in arms
            }
            pose_raw: dict[str, list] = {"lead": [], "trail": []}
            pose_smooth: dict[str, list] = {"lead": [], "trail": []}
            scales: list[float] = []

            for frame in selected:
                image = images.get(frame.frame_index)
                if image is None:
                    continue
                height, width = image.shape[:2]
                pose = _pose_geometry(frame, width, height)
                if pose is None or pose.wrist_separation <= 1.0:
                    continue
                scales.append(pose.shoulder_width)

                raw_frame = raw_by_index.get(frame.frame_index)
                for side in _SIDES:
                    pose_smooth[side].append((frame.frame_index, pose.index(side)))
                    if raw_frame is not None:
                        raw_pose = _pose_geometry(raw_frame, width, height)
                        if raw_pose is not None:
                            pose_raw[side].append((frame.frame_index, raw_pose.index(side)))

                for arm in arms:
                    kind, k = arm
                    box = None if kind == "full" else _crop_box(pose, k or 2.0, width, height)
                    detections = _detect(landmarker, image, box)
                    tally = tallies[arm]
                    tally.frames += 1
                    tally.detections += len(detections)
                    if detections:
                        tally.any_hand += 1
                    if len(detections) == 2:
                        tally.two_hands += 1

                    placed = _assign(detections, pose)
                    tally.placed += sum(1 for d in placed.values() if d is not None)
                    if len(detections) == 2 and all(d is not None for d in placed.values()):
                        tally.distinct += 1
                    for side, (_wrist_lm, _index_lm, expected) in _SIDES.items():
                        det = placed[side]
                        if det is None:
                            continue
                        tally.side_frames[side] += 1
                        if det.label == expected:
                            tally.label_agrees[side] += 1
                        tally.offsets[side].append(
                            _dist(det.points[HAND_WRIST], pose.wrist(side)) / pose.shoulder_width
                        )
                        tally.knuckle_offsets[side].append(
                            _dist(det.points[HAND_INDEX_MCP], pose.index(side))
                            / pose.shoulder_width
                        )
                        tally.spans[side].append(
                            _dist(det.points[HAND_WRIST], det.points[HAND_MIDDLE_TIP])
                            / pose.shoulder_width
                        )
                        tracks[arm][side].append(
                            (frame.frame_index, det.points[HAND_INDEX_MCP])
                        )

            scale = statistics.median(scales) if scales else 1.0
            for arm in arms:
                for side in _SIDES:
                    tallies[arm].steps[side].extend(_step_medians(tracks[arm][side], scale))
                    tallies[arm].pose_steps_raw[side].extend(_step_medians(pose_raw[side], scale))
                    tallies[arm].pose_steps_smooth[side].extend(
                        _step_medians(pose_smooth[side], scale)
                    )
            if args.limit and clips >= args.limit:
                break

    _report(args, clips, tallies)
    return 0


def _fmt(value: float) -> str:
    return "  n/a" if value != value else f"{value:5.2f}"


def _report(args: argparse.Namespace, clips: int, tallies: dict) -> None:
    print(f"\nview={args.view} window={args.window} clips={clips}")
    print(
        f"\n{'arm':>10s}{'frames':>8s}{'>=1':>7s}{'two':>7s}{'placed':>8s}{'distinct':>10s}"
        f"{'lead det':>10s}{'trail det':>11s}{'lead jit':>10s}{'trail jit':>11s}"
    )
    print("-" * 92)
    for (kind, k), tally in tallies.items():
        name = kind if k is None else f"crop k={k}"
        any_rate = tally.any_hand / tally.frames if tally.frames else float("nan")
        print(
            f"{name:>10s}{tally.frames:>8d}{_fmt(any_rate):>7s}{_fmt(tally.detection_rate()):>7s}"
            f"{_fmt(tally.placement_rate()):>8s}{_fmt(tally.distinct_rate()):>10s}"
            f"{_fmt(tally.side_rate('lead')):>10s}{_fmt(tally.side_rate('trail')):>11s}"
            f"{_fmt(tally.jitter('lead', tally.pose_steps_raw)):>10s}"
            f"{_fmt(tally.jitter('trail', tally.pose_steps_raw)):>11s}"
        )

    # The absolute step columns are here because the gated ratio has a denominator that can be
    # tiny: the pose index landmark barely moves at address, so a hand model that is steady in any
    # coaching sense can still divide out to a large number. Added while implementing, before the
    # sweep ran, and reported rather than gated — `thresholds.md` gates the ratio and that stands.
    print("\nrecorded, not gated (steps are median frame-to-frame, in shoulder-widths)")
    print(
        f"{'arm':>10s}{'lead label':>12s}{'trail label':>13s}{'lead off':>10s}{'trail off':>11s}"
        f"{'lead jit/sm':>13s}{'trail jit/sm':>14s}{'lead step':>11s}{'pose step':>11s}"
    )
    print("-" * 105)
    for (kind, k), tally in tallies.items():
        name = kind if k is None else f"crop k={k}"

        def agree(side: str, tally: Tally = tally) -> float:
            n = tally.side_frames[side]
            return tally.label_agrees[side] / n if n else float("nan")

        def offset(side: str, tally: Tally = tally) -> float:
            values = tally.offsets[side]
            return statistics.median(values) if values else float("nan")

        def median(values: list[float]) -> float:
            return statistics.median(values) if values else float("nan")

        print(
            f"{name:>10s}{_fmt(agree('lead')):>12s}{_fmt(agree('trail')):>13s}"
            f"{_fmt(offset('lead')):>10s}{_fmt(offset('trail')):>11s}"
            f"{_fmt(tally.jitter('lead', tally.pose_steps_smooth)):>13s}"
            f"{_fmt(tally.jitter('trail', tally.pose_steps_smooth)):>14s}"
            f"{median(tally.steps['lead']):>11.4f}"
            f"{median(tally.pose_steps_raw['lead']):>11.4f}"
        )

    print(
        f"\nhand shape, recorded not gated (a laid-out hand spans ~{EXPECTED_HAND_SPAN_SW:.2f} sw)"
    )
    print(
        f"{'arm':>10s}{'lead knuckle':>14s}{'trail knuckle':>15s}{'lead span':>11s}"
        f"{'trail span':>12s}"
    )
    print("-" * 62)
    for (kind, k), tally in tallies.items():
        name = kind if k is None else f"crop k={k}"

        def med(values: list[float]) -> str:
            return "  n/a" if not values else f"{statistics.median(values):5.2f}"

        print(
            f"{name:>10s}{med(tally.knuckle_offsets['lead']):>14s}"
            f"{med(tally.knuckle_offsets['trail']):>15s}"
            f"{med(tally.spans['lead']):>11s}{med(tally.spans['trail']):>12s}"
        )

    print(
        f"\ngates: two-hand >= {GATE_DETECTION}, placed/distinct >= {GATE_PLACEMENT}, "
        f"jitter <= {GATE_JITTER} (against raw pose)"
    )
    for (kind, k), tally in tallies.items():
        if kind != "crop":
            continue
        verdicts = []
        for side in _SIDES:
            ok = (
                tally.detection_rate() >= GATE_DETECTION
                and tally.placement_rate() >= GATE_PLACEMENT
                and tally.distinct_rate() >= GATE_PLACEMENT
                and tally.jitter(side, tally.pose_steps_raw) <= GATE_JITTER
            )
            verdicts.append(f"{side}={'PASS' if ok else 'FAIL'}")
        print(f"  crop k={k}: {' '.join(verdicts)}")


def cmd_frames(args: argparse.Namespace) -> int:
    """Dump annotated crops so a human can check the landmarks are on the knuckles.

    A detection rate is a number about boxes; whether landmark 5 is on the index knuckle or
    somewhere on the shaft is a thing only eyes settle. `frames/` is git-ignored.
    """
    import cv2
    from mediapipe.tasks import python as mp_python
    from mediapipe.tasks.python import vision

    view = _VIEWS[args.view]
    options = vision.HandLandmarkerOptions(
        base_options=mp_python.BaseOptions(model_asset_path=str(ensure_hand_model())),
        running_mode=vision.RunningMode.IMAGE,
        num_hands=2,
    )
    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    written = 0

    with vision.HandLandmarker.create_from_options(options) as landmarker:
        for keypoints_path in sorted(settings.sessions_dir.rglob(f"{view.stem}.keypoints.json")):
            swing_dir = keypoints_path.parent
            if args.swing and args.swing not in str(swing_dir):
                continue
            clip = load_clip(swing_dir, view)
            video = _video_for(swing_dir, view.stem)
            if clip is None or video is None:
                continue
            lo, hi = clip.address
            middle = clip.frames[(lo + hi) // 2]
            images = _read_range(video, middle.frame_index, middle.frame_index)
            image = images.get(middle.frame_index)
            if image is None:
                continue
            height, width = image.shape[:2]
            pose = _pose_geometry(middle, width, height)
            if pose is None:
                continue

            box = _crop_box(pose, args.k, width, height)
            detections = _detect(landmarker, image, box)
            x0, y0, x1, y1 = box
            patch = cv2.resize(image[y0:y1, x0:x1], (CROP_PX, CROP_PX))
            sx, sy = CROP_PX / (x1 - x0), CROP_PX / (y1 - y0)
            for det in detections:
                for idx, (px, py) in enumerate(det.points):
                    colour = (0, 255, 0) if det.label == "Left" else (0, 128, 255)
                    cv2.circle(patch, (int((px - x0) * sx), int((py - y0) * sy)), 3, colour, -1)
                    if idx in (HAND_WRIST, HAND_INDEX_MCP, HAND_PINKY_MCP):
                        cv2.putText(
                            patch,
                            str(idx),
                            (int((px - x0) * sx) + 4, int((py - y0) * sy) - 4),
                            cv2.FONT_HERSHEY_SIMPLEX,
                            0.4,
                            colour,
                            1,
                        )
            for side in _SIDES:
                wx, wy = pose.wrist(side)
                cv2.drawMarker(
                    patch, (int((wx - x0) * sx), int((wy - y0) * sy)), (255, 255, 255),
                    cv2.MARKER_CROSS, 14, 1,
                )
            name = f"{clip.label.replace('/', '-')}-k{args.k}-{len(detections)}hands.png"
            cv2.imwrite(str(OUT_ROOT / name), patch)
            written += 1

    print(f"wrote {written} crops to {OUT_ROOT}")
    print("white crosses are the pose wrists; dots are the hand model (green=Left, orange=Right)")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    measure = sub.add_parser("measure", help="the numbers in thresholds.md")
    measure.add_argument("--view", choices=sorted(_VIEWS), default="face-on")
    measure.add_argument("--window", choices=("address", "clip"), default="address")
    measure.add_argument("--arm", choices=("crop", "full", "both"), default="crop")
    measure.add_argument(
        "--k", type=float, default=None, help="one crop scale instead of the sweep"
    )
    measure.add_argument("--swing", default=None, help="substring match on the swing directory")
    measure.add_argument("--limit", type=int, default=None, help="stop after N clips")
    measure.add_argument(
        "--min-confidence",
        type=float,
        default=0.5,
        help="post-hoc diagnostic; the gates ran at the MediaPipe default of 0.5",
    )
    measure.set_defaults(func=cmd_measure)

    frames = sub.add_parser("frames", help="annotated crops for eyeballing")
    frames.add_argument("--view", choices=sorted(_VIEWS), default="face-on")
    frames.add_argument("--k", type=float, default=2.0)
    frames.add_argument("--swing", default=None)
    frames.set_defaults(func=cmd_frames)

    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
