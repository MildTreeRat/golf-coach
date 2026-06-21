"""Dev CLI: run MediaPipe pose over a video file and write a skeleton-overlay clip. [M1]

Usage (once M1 is implemented):
    python scripts/run_pose.py data/raw/my_swing.mp4

This is the first visible, hardware-free milestone artifact (ROADMAP M1).
"""

from __future__ import annotations

import sys


def main(argv: list[str]) -> int:
    raise NotImplementedError("M1: wire FileVideoSource -> estimate_pose -> overlay render.")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
