# Spikes

Throwaway, exploratory code — **not** part of the `golf_coach` package and not held to
the same quality bar. Use this for time-boxed investigations, then fold the *findings*
(not the code) into an ADR or the real package.

Each spike gets its own dated folder.

## Planned / active

- `club-head-detectability/` — **Milestone 1.5** (ROADMAP). Before labeling 200–500
  images for YOLOv8, prove the club head is detectable through the impact zone. Capture a
  few swing clips (phone is fine), eyeball the impact frames, test lighting + fast shutter,
  and decide the strategy: pure-ML / marker-assisted / fusion+interpolation. Exit = a
  go/no-go decision recorded as an ADR.
