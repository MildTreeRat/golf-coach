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
