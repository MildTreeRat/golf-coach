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
- `club-head-detectability/` — **Milestone 1.5**, **run 2026-08-14 and closed**. Needed no new
  footage: the four bay clips already on disk were enough. Contains `thresholds.md` (committed
  before any frame was extracted), `probe.py` (`frames` and `measure`), and `log.md`. The club
  head is a good target at rest — 42 px at 4K — and a translucent 600–980 px band at impact, so
  **pure-ML is a no-go and a marker does not fix it**: the binding constraint is exposure time,
  measured at **~1/2000 s**. Findings: [log.md](club-head-detectability/log.md). Decision:
  [ADR-017](../docs/decisions/017-club-head-detection-strategy.md).
  `frames/` is git-ignored and regenerates from `probe.py frames`.
