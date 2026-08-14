"""M1.5 club-head detectability probe — THROWAWAY. Not part of the `golf_coach` package.

Answers the M1.5 spike question against the four real bay clips already on disk (see
spikes/README.md and thresholds.md, which was committed *before* any of this ran):

    Is the club head a bounded, localizable object through the impact zone, or an
    unlabelable smear?

**Nothing here re-derives an impact frame.** The shipped pipeline already found impact on all
four clips and stored it in `analysis.json`; re-detecting it here would risk measuring a
different frame than the one the repo scores, and the whole point is to inspect *the frames the
system actually calls impact*. Same reasoning as the M7 Phase 0 probe reaching into
`analysis/phases.py` privates: measure the shipped thing, not a re-implementation.

Subcommands::

    probe.py frames  [--swing S] [--view V] [--pad N]  # index-burned frames + impact-zone crops
    probe.py measure [--swing S]                       # the numbers in thresholds.md

Both need the `vision` extra (OpenCV). `measure` prints a table and writes nothing.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
OUT_ROOT = Path(__file__).resolve().parent / "frames"

# MediaPipe landmark indices. Duplicated rather than imported so `frames`/`measure` run without
# the package installed — a spike that can't run on a bare checkout is a spike nobody re-runs.
L_SHOULDER, R_SHOULDER = 11, 12
L_WRIST, R_WRIST = 15, 16
L_ANKLE, R_ANKLE = 27, 28

# Two scale assumptions, in metres, stated here so they are easy to re-run with measured values.
#
# SHOULDER_WIDTH_M is the repo's usual normalizer and it is **face-on only**. Measured
# down-the-line it reads 112 px/m against face-on's 883 — an 8x disagreement, because from
# behind the golfer the shoulders are edge-on and biacromial width projects to nearly nothing.
# That is the same failure M6.5 found in `head_hip_offset_impact_norm`: a quantity comparing two
# body parts at one instant does not survive a change of camera yaw. Kept as a face-on
# cross-check only.
#
# SHOULDER_TO_ANKLE_M is the scale everything downstream uses, because it is **vertical** and
# yaw leaves vertical distances alone — the same ruler works in both views by construction.
SHOULDER_WIDTH_M = 0.40
SHOULDER_TO_ANKLE_M = 1.40

# The scale that actually matters, and the reason the two above are only context. The club head
# at impact is in the **ball's** plane, which is nearer the camera than the golfer's body — the
# ball measures ~57 px where the body-plane ruler predicts 34, so the ball plane runs about 1.66x
# the body scale. Using the body scale would under-state every blur figure by that factor.
#
# Hand-measured off a 3x zoom with a 20 px grid (`frames/*-ball-3x-grid20.png`), face-on address
# frame, both swings independently: 56 px and 57 px for a ball that is 42.7 mm across. Hand
# measurement is the honest method here — automatic thresholding kept latching onto bright
# patches of the mat.
BALL_DIAMETER_M = 0.0427
BALL_PX = {"aaron-1": 56, "aaron-2": 57}

# Club head at rest, face-on, aaron-2, measured the same way: 42 px across the short axis
# (the head seen near-edge-on from the heel) by 97 px along the blade.
HEAD_PX_AT_REST = 42

# Motion mask: an 8-bit difference above this is "moved". Set from the noise floor of a static
# patch on this footage (measured ~6-9 counts of sensor noise between adjacent frames).
_DIFF_THRESHOLD = 25

SWINGS = {
    "aaron-1": "data/processed/sessions/2026-08-07-aaron1/1",
    "aaron-2": "data/processed/sessions/2026-08-10/2",
}
VIEWS = ("face_on", "down_the_line")


def _clip_paths(swing: str, view: str) -> tuple[Path, Path]:
    """(video, keypoints) for one view of one swing."""
    session = REPO / SWINGS[swing]
    videos = sorted(session.glob(f"{view}.*.MOV")) + sorted(session.glob(f"{view}.*.mov"))
    if not videos:
        raise SystemExit(f"no {view} video under {session}")
    return videos[0], session / f"{view}.keypoints.json"


def _anchors(swing: str, view: str) -> dict:
    """Phase instants as the shipped pipeline stored them. `a` is face-on, `b` down-the-line."""
    analysis = json.loads((REPO / SWINGS[swing] / "analysis.json").read_text())
    side = "a" if view == "face_on" else "b"
    return analysis["alignment"][side]["anchors"]


def _shot(swing: str) -> dict | None:
    return json.loads((REPO / SWINGS[swing] / "analysis.json").read_text())["swing"].get("shot")


def _landmarks(keypoints_path: Path, index: int) -> list[dict]:
    frames = json.loads(keypoints_path.read_text())["frames"]
    return frames[index]["landmarks"]


def _impact_zone(landmarks: list[dict], width: int, height: int) -> tuple[int, int, int, int]:
    """The box the club head must be inside at impact: hands down to the ground.

    Derived from the golfer, not from a guess about where the ball is — the wrists give the
    horizontal centre and the ankles give the floor, so the crop follows the golfer between
    clips and between swings instead of being hand-tuned per clip.
    """
    wx = (landmarks[L_WRIST]["x"] + landmarks[R_WRIST]["x"]) / 2 * width
    wy = (landmarks[L_WRIST]["y"] + landmarks[R_WRIST]["y"]) / 2 * height
    ay = max(landmarks[L_ANKLE]["y"], landmarks[R_ANKLE]["y"]) * height
    # The floor sits *below* the ankle landmark (MediaPipe puts it at the joint, ~8 cm up), and
    # the ball and the club head sit on the floor. A crop ending at the ankle cuts the head off
    # exactly where it matters — the first run of this probe did, which is why the margin is
    # this generous.
    half = 0.26 * width
    x0 = int(max(0, wx - half))
    x1 = int(min(width, wx + half))
    y0 = int(max(0, wy - 0.03 * height))
    # ...and 0.09 was still not enough. The phone looks slightly down, so the ball — nearer the
    # camera than the golfer's feet — projects *below* the feet in image space. At address the
    # head sits about a fifth of a frame height under the ankle landmark.
    y1 = int(min(height, ay + 0.20 * height))
    return x0, y0, x1, y1


def _px_per_m(landmarks: list[dict], width: int, height: int) -> float:
    """Pixels per metre from the golfer's vertical extent — works in both views (see above)."""
    sy = (landmarks[L_SHOULDER]["y"] + landmarks[R_SHOULDER]["y"]) / 2 * height
    ay = max(landmarks[L_ANKLE]["y"], landmarks[R_ANKLE]["y"]) * height
    return abs(ay - sy) / SHOULDER_TO_ANKLE_M


def _px_per_m_shoulders(landmarks: list[dict], width: int, height: int) -> float:
    """Face-on-only cross-check on `_px_per_m`. Meaningless down-the-line."""
    dx = (landmarks[L_SHOULDER]["x"] - landmarks[R_SHOULDER]["x"]) * width
    dy = (landmarks[L_SHOULDER]["y"] - landmarks[R_SHOULDER]["y"]) * height
    return math.hypot(dx, dy) / SHOULDER_WIDTH_M


def _read_range(video: Path, first: int, last: int) -> dict[int, "object"]:
    """Decode [first, last] sequentially and stop.

    Sequential, not `CAP_PROP_POS_FRAMES` seeking: HEVC seeks land on the nearest keyframe and
    silently return a neighbouring frame, and every index here came from a pipeline that counted
    frames from zero. An off-by-three would put the "impact" label on the wrong picture.
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


def cmd_frames(args: argparse.Namespace) -> int:
    import cv2

    for swing in ([args.swing] if args.swing else list(SWINGS)):
        for view in ([args.view] if args.view else VIEWS):
            video, kp = _clip_paths(swing, view)
            anchors = _anchors(swing, view)
            impact, top = anchors["impact"], anchors["top"]
            start = anchors["motion_start"]
            lo, hi = impact - args.pad, impact + args.pad
            wanted = sorted({start, top, *range(lo, hi + 1)})

            images = _read_range(video, min(wanted), max(wanted))
            out_dir = OUT_ROOT / f"{swing}-{view}"
            out_dir.mkdir(parents=True, exist_ok=True)

            height, width = images[impact].shape[:2]
            zone = _impact_zone(_landmarks(kp, impact), width, height)
            x0, y0, x1, y1 = zone

            written = 0
            for index in wanted:
                image = images.get(index)
                if image is None:
                    continue
                label = {start: "address", top: "top", impact: "IMPACT"}.get(index, "")
                crop = image[y0:y1, x0:x1].copy()
                cv2.putText(
                    crop, f"{index} {label}", (16, 56),
                    cv2.FONT_HERSHEY_SIMPLEX, 1.6, (0, 0, 255), 4, cv2.LINE_AA,
                )
                cv2.imwrite(str(out_dir / f"zone-{index:05d}.png"), crop)
                written += 1

            print(f"{swing}/{view}: {written} impact-zone crops "
                  f"({x1 - x0}x{y1 - y0} px, native) -> {out_dir}")
    return 0


def cmd_measure(args: argparse.Namespace) -> int:
    import cv2
    import numpy as np

    # `blob` is the largest connected component of the INTER-frame difference mask. It is not
    # the club head and not the club head's intra-frame smear: it spans whatever moved between
    # two frames, body included. Reported as observational context only — the decision numbers
    # are in the summary, which derives from speed and scale rather than from this mask.
    print(f"{'clip':<26} {'frame':>6} {'rel':>4} {'move%':>7} {'blob':>7} "
          f"{'sharp':>6} {'ratio':>6}")
    print("-" * 72)

    summary: list[dict] = []
    for swing in ([args.swing] if args.swing else list(SWINGS)):
        for view in VIEWS:
            video, kp = _clip_paths(swing, view)
            anchors = _anchors(swing, view)
            impact = anchors["impact"]
            address = anchors["motion_start"]
            lo, hi = impact - args.pad, impact + args.pad

            images = _read_range(video, min(address, lo - 1), hi)
            height, width = images[impact].shape[:2]
            marks = _landmarks(kp, impact)
            x0, y0, x1, y1 = _impact_zone(marks, width, height)
            ppm = _px_per_m(marks, width, height)

            def roi(index: int):
                image = images.get(index)
                return None if image is None else image[y0:y1, x0:x1]

            base = roi(address)
            base_gray = cv2.cvtColor(base, cv2.COLOR_BGR2GRAY)
            base_sharp = float(cv2.Laplacian(base_gray, cv2.CV_64F).var())

            worst = None
            for index in range(lo, hi + 1):
                cur, prev = roi(index), roi(index - 1)
                if cur is None or prev is None:
                    continue
                gray = cv2.cvtColor(cur, cv2.COLOR_BGR2GRAY)
                pgray = cv2.cvtColor(prev, cv2.COLOR_BGR2GRAY)
                diff = cv2.absdiff(gray, pgray)
                mask = (diff > _DIFF_THRESHOLD).astype("uint8")
                mask = cv2.morphologyEx(
                    mask, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5))
                )
                moved = float(mask.mean())

                # Largest moving component: bounded object or unbounded streak?
                count, _, stats, _ = cv2.connectedComponentsWithStats(mask, connectivity=8)
                streak = 0
                if count > 1:
                    biggest = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA]))
                    streak = int(max(stats[biggest, cv2.CC_STAT_WIDTH],
                                     stats[biggest, cv2.CC_STAT_HEIGHT]))

                lap = cv2.Laplacian(gray, cv2.CV_64F)
                mpx = lap[mask.astype(bool)]
                spx = lap[~mask.astype(bool)]
                sharp = float(mpx.var()) if mpx.size > 32 else 0.0
                static = float(spx.var()) if spx.size > 32 else 0.0
                ratio = sharp / static if static > 0 else 0.0

                rel = index - impact
                print(f"{swing + '/' + view:<26} {index:>6} {rel:>+4} {moved * 100:>6.2f}% "
                      f"{streak:>7} {sharp:>6.0f} {ratio:>6.2f}")
                if worst is None or abs(rel) <= 1 and ratio < worst["ratio"]:
                    worst = {"rel": rel, "ratio": ratio, "streak": streak}

            summary.append({
                "clip": f"{swing}/{view}", "ppm": ppm,
                "ppm_shoulders": _px_per_m_shoulders(marks, width, height),
                "base_sharp": base_sharp, "fps": anchors["fps"],
                "zone": (x1 - x0, y1 - y0), "worst": worst, "swing": swing,
            })
            print("-" * 72)

    # Everything below derives from the launch monitor's own club-head speed, the clip's frame
    # rate and the pixel scale. None of it comes from the difference mask above, because a mask
    # built from two frames measures motion *between* them and says nothing about how far the
    # head smears *within* one exposure.
    print("\nScale, sampling and the exposure that would be needed\n" + "=" * 72)
    head_m = 0.08  # heel-to-toe of a mid-iron; a driver is ~0.115 m
    seen: set[str] = set()
    for row in summary:
        swing = row["swing"]
        print(f"{row['clip']:<26} zone {row['zone'][0]}x{row['zone'][1]} px | body plane "
              f"{row['ppm']:.0f} px/m (vertical), {row['ppm_shoulders']:.0f} px/m (shoulders)")
        if swing in seen:
            continue
        seen.add(swing)

        shot = _shot(swing)
        speed_mph = (shot or {}).get("club_head_speed")
        ball_speed = (shot or {}).get("ball_speed")
        ppm = BALL_PX[swing] / BALL_DIAMETER_M
        fps = row["fps"]
        print(f"{'':<26} BALL PLANE {ppm:.0f} px/m (ball {BALL_PX[swing]} px / "
              f"{BALL_DIAMETER_M * 1000:.1f} mm) -- the plane the head is in at impact")
        print(f"{'':<26} head at rest {HEAD_PX_AT_REST} px short axis "
              f"= {HEAD_PX_AT_REST / ppm * 100:.1f} cm")
        if not speed_mph:
            continue

        # The launch monitor's own club-speed reading is not trustworthy on these two shots:
        # smash factor = ball speed / club speed comes out at or below 1.0, and a real strike
        # cannot exceed ~1.5 or plausibly fall below ~1.1. So the true club speed is bracketed
        # rather than assumed, and the verdict is checked across the whole bracket.
        smash = ball_speed / speed_mph if ball_speed and speed_mph else None
        low_mph = (ball_speed / 1.5) if ball_speed else speed_mph
        print(f"{'':<26} shot says club {speed_mph:g} mph, ball {ball_speed:g} mph "
              f"-> smash {smash:.2f}" + ("  <-- IMPLAUSIBLE, so speed is bracketed"
                                         if smash and smash < 1.1 else ""))
        for label, mph in (("low", low_mph), ("as read", speed_mph)):
            speed = mph * 0.44704
            travel = speed / fps
            needed = (HEAD_PX_AT_REST / 2 / ppm) / speed
            print(f"{'':<26}   {label:<8} {mph:5.1f} mph = {speed:4.1f} m/s | "
                  f"between frames {travel * 100:5.1f} cm = {travel * ppm:5.0f} px "
                  f"= {travel / head_m:4.1f} head-lengths")
            print(f"{'':<26}   {'':<8} smear at 1/60 s {speed / 60 * ppm:5.0f} px | "
                  f"1/1000 s {speed / 1000 * ppm:4.0f} px | "
                  f"1/2000 s {speed / 2000 * ppm:4.0f} px")
            print(f"{'':<26}   {'':<8} exposure to keep smear under half a head "
                  f"({HEAD_PX_AT_REST // 2} px): 1/{1 / needed:.0f} s")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    frames = sub.add_parser("frames", help="dump impact-zone crops at native resolution")
    frames.add_argument("--swing", choices=sorted(SWINGS))
    frames.add_argument("--view", choices=VIEWS)
    frames.add_argument("--pad", type=int, default=6)
    frames.set_defaults(func=cmd_frames)

    measure = sub.add_parser("measure", help="motion, streak and sharpness through impact")
    measure.add_argument("--swing", choices=sorted(SWINGS))
    measure.add_argument("--pad", type=int, default=4)
    measure.set_defaults(func=cmd_measure)

    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
