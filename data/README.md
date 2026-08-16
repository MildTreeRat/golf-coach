# data/

Local data — **gitignored** (videos, models, and the DB are large/binary). This README
keeps the directory in git and documents the layout.

```
data/
├── raw/         # raw swing video files, e.g. raw/sessions/{session_id}/   (gitignored)
├── processed/   # extracted keypoints, labeled images, intermediate artifacts (gitignored)
│   ├── sessions/      # one directory per swing bundle + its analysis artifacts
│   ├── shots/         # parsed launch-monitor shots, content-addressed
│   ├── golfers/       # the golfer registry, one file per golfer
│   └── conversations/ # follow-up conversation transcripts, one per conversation (ADR-020)
├── models/      # trained model weights, *.pt/*.onnx                        (gitignored)
├── reference/   # third-party reference corpora — see below                 (gitignored)
└── golf_trainer.db   # SQLite database                                      (gitignored)
```

`conversations/` holds the model's own content blocks verbatim — thinking blocks included —
because they are replayed to the API on the next turn rather than merely displayed. Treat one as
a transcript of a private coaching conversation, not as a log: it quotes the golfer's questions
and the swing's measurements in full.

Drop a sample/phone swing clip into `data/raw/` to start Milestone 1.

## `reference/` — third-party corpora (M4-REF)

Populated by `scripts/golfdb/*`. **Gitignored for licensing as much as for size**: GolfDB's code is
CC BY-NC 4.0, the dataset states no license, and the clips are third-party broadcast footage. Only
*aggregate* statistics are committed, and they live in the package at
`src/golf_coach/analysis/benchmarks/golfdb_v1.json`. See [ADR-012](../docs/decisions/012-golfdb-reference-data.md).

```
reference/golfdb/
├── upstream/                  # golfDB.pkl / .mat, fetched from the upstream repo
├── swings.jsonl               # Tier 2: one ReferenceSwing per clip (~1 MB, 1399 rows)
└── keypoints/<estimator>/     # Tier 1: per-clip keypoints, one dir per pose estimator
```

Three tiers with different lifetimes:

- **Tier 1** is a **cache, not a source of truth** — expensive to produce (an estimator over ~170k
  frames), fully reproducible from the clips. Kept so a future checkpoint can be measured across a
  thousand tour swings without touching a video. Same serialization as
  `data/processed/*.keypoints.json`, just compact and rounded.
- **Tier 2** is one row per clip: metadata, ground-truth event frames, and every derived metric.
  This is the layer benchmark bands are re-cut from. Its shape is deliberately the shape the future
  SQLite `swings` table will take.
- **Tier 3** — the committed aggregates — is the only part that ships.

Rebuild from scratch:

```
python scripts/golfdb/fetch.py
python scripts/golfdb/ingest_labels.py
python scripts/golfdb/extract_pose.py --videos <videos_160 dir>   # optional, slow
python scripts/golfdb/derive_pose_metrics.py
python scripts/golfdb/derive_reference.py
```

The clip archive (`videos_160`, ~700 MB) is downloaded separately per the upstream README and is
**not** stored here.

## `reference/caddieset/` — the paired corpus (M8-PAIR)

Populated by `scripts/caddieset/*`. The one corpus here with **mechanics and ball flight on the same
row**: 1,757 rows (924 face-on) from eight golfers of mixed skill, per-phase joint metrics across
the same eight swing events GolfDB annotates, plus the launch monitor's reading. See
[ADR-021](../docs/decisions/021-caddieset-paired-reference-data.md).

```
reference/caddieset/
├── upstream/CaddieSet.csv   # the shipped file, one CSV, ~500 KB
├── upstream/LICENSE         # MIT — fetched alongside so the terms travel with the data
└── shots.jsonl              # one row per view-of-a-shot, cells that cannot be trusted made absent
```

**MIT, so this one could have been vendored into git — and deliberately is not.** Keeping every
third-party corpus under one gitignored root makes the licensing boundary a directory rather than a
per-file judgement someone has to remember. One HTTP GET rebuilds it.

Two things to know before using it. Its joint metrics are CaddieSet's own definitions, **not** our
`metric_definitions_version: 3`, so no value here may ever become a band (ADR-012 §4). And 803
face-on rows share a launch-monitor reading with a down-the-line row, so 1,757 rows are roughly 950
physical shots — `common.shot_key` is what makes that visible instead of double-counted.

```
python scripts/caddieset/fetch.py
python scripts/caddieset/ingest.py
python scripts/caddieset/study_panel.py   # needs the `research` extra
```
