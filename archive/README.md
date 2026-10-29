# `archive/` — the research record

**Tier: AS-BUILT.** What is here, what it needs, and until when it runs. Everything here moved from
`scripts/` and `src/golf_coach/contracts/` at M29
([ADR-035 §5](../docs/decisions/035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)),
and none of it is ported.

## What is here

| Path | What it is |
|---|---|
| `golfdb/` | GolfDB's fetch, pose extraction, band and model derivation, the pose bake-off and the `tune_*` gates — every losing candidate kept runnable ([ADR-012](../docs/decisions/012-golfdb-reference-data.md), [ADR-022](../docs/decisions/022-learned-artifacts-as-committed-data.md), [M4_POSE_BAKEOFF](../docs/M4_POSE_BAKEOFF.md)) |
| `golfdb/reference.py` | `ReferenceSwing`, the per-clip row a band is cut from. It was `contracts/reference.py`, and it moved beside its only importer |
| `caddieset/` | CaddieSet's ingestion and the M8-PAIR study ([ADR-021](../docs/decisions/021-caddieset-paired-reference-data.md)) |
| `hand_landmark_reliability.py` | M14's gate: can hand landmarks be measured at address? ([M14_HAND_LANDMARKS](../docs/M14_HAND_LANDMARKS.md)) |
| `trigger_replay.py` | M20's harness: the stored clips through the release `golf-trigger` |
| `pose_replay.py` | M23's harness: the stored clips re-posed through the release `golf-pose` and diffed against the keypoints on disk ([ADR-033](../docs/decisions/033-the-pose-sidecar-protocol.md)) |

**Why archived, and not ported or deleted.** Retired Python whose Rust twin conforms is deleted
(ADR-035 §2), because a copy that nothing runs is a copy that drifts. These have no twin and none is
planned: fitting happens offline, and what it produces ships as committed JSON (ADR-022). So the
research record comes here instead.

**What they produced stays where it is.** The benchmark JSON lives in
`src/golf_coach/analysis/benchmarks/`, and `crates/analysis` reads it there by `include_str!` until
M40 moves it crates-side.

## Running it

- **From the repo root, with the package installed in `.venv`**:
  `.venv/Scripts/python.exe archive/golfdb/<script>.py`. Running a script puts its own directory on
  `sys.path`, and that is how `import common` and `from reference import ReferenceSwing` resolve.
  `archive/` itself is not a package.
- **Third-party libraries are installed by hand.** The `research` extra that carried pandas, numpy
  and scikit-learn went with the move: `.venv/Scripts/python.exe -m pip install pandas numpy
  scikit-learn`. Pose extraction needs MediaPipe, from the `vision` extra. The two replay harnesses
  drive the release binaries, so `cargo build --release` comes first.
- **Nothing lints or type-checks it.** `ruff check src tests scripts` and `mypy src` do not reach
  here. It was ruff-clean when it moved.

## Until when

**Much of `golfdb/` imports `golf_coach.analysis`, so it runs until M40 deletes `analysis/`**, and is
a record after that. Even now it can re-derive nothing that Rust has changed: from M32 a metric's
definition changes in Rust only, and the archive measures with frozen Python. So the first change
that needs a re-derive needs a decision first
([ADR-022's M31.5 addendum](../docs/decisions/022-learned-artifacts-as-committed-data.md#addendum-2026-09-30-m315-fitting-is-archived-the-artifacts-stay-and-rust-evaluates-them)).

`spikes/` stays where it was (ADR-035 §5), as history. `spikes/mediapipe-hands/probe.py` imports
`hand_landmark_reliability.py` from here.
