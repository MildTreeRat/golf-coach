# Spikes

Throwaway, exploratory code — **not** part of the `golf_coach` package and not held to
the same quality bar. Use this for time-boxed investigations, then fold the *findings*
(not the code) into an ADR or the real package.

Each spike gets its own dated folder.

## Planned / active

- `2026-08-07-two-phone/` — **M7 Phase 0** field spike. Does `segment_phases()` find sane
  top/impact on **down-the-line** footage (it tracks `LEFT_WRIST`, tuned on 461 *face-on*
  clips)? Does OpenCV decode iPhone HEVC? Does `CAP_PROP_FPS` describe real time on a
  slo-mo clip? Contains `probe.py` (reaches into `analysis/phases.py` privates on purpose —
  it must measure the *shipped* rule), `log.md` for the bay session, and `truth.json` for
  hand labels. Pass/fail thresholds were committed **before** any footage was recorded.
  Findings: [docs/M7_TWO_PHONE_SPIKE.md](../docs/M7_TWO_PHONE_SPIKE.md).
- `club-head-detectability/` — **Milestone 1.5** (ROADMAP). Before labeling 200–500
  images for YOLOv8, prove the club head is detectable through the impact zone. Capture a
  few swing clips (phone is fine), eyeball the impact frames, test lighting + fast shutter,
  and decide the strategy: pure-ML / marker-assisted / fusion+interpolation. Exit = a
  go/no-go decision recorded as an ADR.
