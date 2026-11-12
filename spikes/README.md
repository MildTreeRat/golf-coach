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
- `mediapipe-hands/` — **M14's successor**, **run 2026-09-03 and closed**. Needed no new footage:
  the fifteen face-on swings on disk were enough. Contains `thresholds.md` (committed before the
  model was run over a frame), `probe.py` (`measure` and `frames`) and `log.md`. M14 P3 proved the
  *coarse* pose hand points are measurable face-on at address and left the 21-point model as the
  only route to grip **strength**; this ran it. **No-go, and not for the expected reason** — the
  model returns a correctly-placed hand on 179 of 179 address frames and resolves **two** hands on
  15% of them, because a golf grip reads as one hand. The one it keeps is the **trail** hand, and
  grip strength is read on the lead. Findings: [log.md](mediapipe-hands/log.md). `frames/` is
  git-ignored and regenerates from `probe.py frames`.
- `vision-ocr/` — **M33**, **run 2026-11-04 on pasted copies, and still open**. Does Apple Vision produce boxes
  that `golf-core parse-screen`, the parser the phone will call, turns into correct shots of Full Swing's grid
  screen? Contains `thresholds.md` (committed before Vision read one real photo), `vision_ocr.swift` (the
  recognizer and the rectifier stand-in), `probe.py` (`build-manifest`, `check`, `prepare`, `control`,
  `measure`, `failures`, `calibrate` and `verdict`), `lab-images/` (the lab's working images, a Rust crate of
  its own), `photos.json`, `truth_full_swing.json`, `captures/` (the JPEGs saved from a chat paste), `boxes/`,
  `results/` and `log.md`. **GO, provisional: pasted copies. Clips: not measured.** At the first settings, with
  no parser rule, the photo as imported read 71 of 72 values with none wrong, and it is at one bar's limit; the
  VisionKit stand-in found the wrong rectangle or none, which says nothing about VisionKit. **This is not yet
  the answer on the photos as captured**: the original files, the five clips, HD Golf (the user's omission) and
  the lab's reader have not run, and whether M38 is unblocked is the user's. Findings:
  [log.md](vision-ocr/log.md), which `probe.py verdict` holds to `results/`. `frames/` is git-ignored and
  regenerates from `probe.py prepare` and `measure`.
- `strike-tape/` — **M46**, **drawn 2026-11-12, and open until the tape session (the plan's P3) runs**.
  Not a probe: `truth-sheet.svg` is the printed millimetre grid that peeled STRIKEPRO labels are stuck on
  and photographed, as the truth the drill's tape reader is measured against
  ([the plan](../docs/plans/m46-impact-drill.md)'s D7). `README.md` holds the sheet's geometry, which
  P13's truth reader rectifies to, and the session is
  [the run sheet](../docs/plans/m46-tape-run-sheet.md). The photos stay in git-ignored
  `data/raw/strike_tape/`.
