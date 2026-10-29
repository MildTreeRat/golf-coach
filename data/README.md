# data/

Local data — **gitignored** (videos, models, and the DB are large/binary). This README
keeps the directory in git and documents the layout.

```
data/
├── raw/         # raw swing video files, e.g. raw/sessions/{session_id}/   (gitignored)
├── processed/   # extracted keypoints, labeled images, intermediate artifacts (gitignored)
│   ├── sessions/      # one directory per swing bundle + its analysis artifacts, under a
│   │                  #   session.json holding two cursors, golfer and club (ADR-024), and a
│   │                  #   third, intent, that only Rust writes (M35)
│   ├── shots/         # parsed launch-monitor shots, content-addressed
│   ├── golfers/       # per golfer: the registry record, and the declared bag beside it
│   └── conversations/ # follow-up conversation transcripts, one per conversation (ADR-020)
├── models/      # trained model weights, *.pt/*.onnx                        (gitignored)
│   └── ocr/           # the lab reader's three ONNX models and the recognizer's dictionary
│                      #   (M29), each sha-pinned by crates/ocr/models.json
├── reference/   # third-party reference corpora — see below                 (gitignored)
└── golf_trainer.db   # SQLite database                                      (gitignored)
```

**Since M29 the Rust lab writes `processed/`**, and the frozen FastAPI server's routes that wrote it
answer 410 naming the verb that does it now (ADR-035's M29 addendum). `golf-lab analyze` and
`reanalyze` write a swing's artifacts, `golf-lab import-photos` writes `shots/` (and a photo entry
with `--into-session`), and `golf-lab import-export` merges a phone's export of `sessions/`,
`shots/` and `golfers/` in file by file, the phone's own bytes, refusing the whole import on any file
that would change. M29 P29 re-read and stamped the 13 stored shots and re-ran the 15 stored bundles
at engine 17, after backing `processed/` up without the clips to
`%USERPROFILE%\golf-coach-backups\m29-pre-write\` with a `SHA256SUMS` file; that backup is the
record of `data/` before Rust wrote it, and nothing deletes it.

A swing directory under `sessions/` can hold two files only Rust writes (M35). Neither exists on
the stored bundles yet, and every write test runs on a scratch copy:

- **`intent` in `manifest.json`** (and the matching cursor in `session.json`): what the shot was
  practised as, written only when set, by `golf-core set-intent` or stamped from the cursor at
  upload. A `drill` is stored and analysed and counts toward no club or player stat. **Frozen
  Python reads the key and drops it on any rewrite** of that file, so a recorded drill becomes a
  tracked shot the next time the frozen server writes to the swing (ADR-024's M35 addendum).
- **`shot_analysis.json`**: a photo entry's analysis (a shot photo with no face-on clip), a
  `ShotResult` written by `golf-core run-shot --session` beside the manifest. That verb reads
  `shots/`, found through `--shots-dir`, else `GOLF_SHOTS_DIR`, else `data/processed/shots`. A swing
  with a clip never gets one: its shot is analysed with it, into `analysis.json`.

`golfers/` holds two record kinds per player, kept apart by suffix: `<player_id>.golfer.json`
(name and handedness) and `<player_id>.bag.json` (the declared bag — ADR-024). A bag is the one
thing here that is typed in rather than recomputed from the clips, so deleting one loses lofts
that cannot be recovered from anything else on disk.

`conversations/` holds the model's own content blocks verbatim — thinking blocks included —
because they are replayed to the API on the next turn rather than merely displayed. Treat one as
a transcript of a private coaching conversation, not as a log: it quotes the golfer's questions
and the swing's measurements in full.

Drop a sample/phone swing clip into `data/raw/` to start Milestone 1.

## `models/ocr/` — the lab's screen reader (M29)

The three PaddleOCR models `crates/ocr` runs through ONNX Runtime, one directory each, with
`model.onnx` and the `inference.yml` it was converted beside: `PP-OCRv6_medium_det`,
`PP-LCNet_x1_0_textline_ori` and `PP-OCRv6_medium_rec`, whose directory also holds `dict.txt`, the
recognizer's dictionary. **Gitignored, and verified rather than trusted**: `crates/ocr/models.json`
is committed and pins each file's sha256, its source model and its preprocessing constants, and the
reader refuses a missing file or a digest that disagrees, by name. They were converted once from
PaddleX's models with the toolchain the manifest's `conversion` block records (ADR-035's M29
addendum), and a reconversion matched when its digests do. Nothing else under `data/` is read by the
reader but the photos.

## `reference/` — third-party corpora (M4-REF)

Populated by `archive/golfdb/*`. **Gitignored for licensing as much as for size**: GolfDB's code is
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
python archive/golfdb/fetch.py
python archive/golfdb/ingest_labels.py
python archive/golfdb/extract_pose.py --videos <videos_160 dir>   # optional, slow
python archive/golfdb/derive_pose_metrics.py
python archive/golfdb/derive_reference.py
```

The clip archive (`videos_160`, ~700 MB) is downloaded separately per the upstream README and is
**not** stored here.

## `reference/caddieset/` — the paired corpus (M8-PAIR)

Populated by `archive/caddieset/*`. The one corpus here with **mechanics and ball flight on the same
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
python archive/caddieset/fetch.py
python archive/caddieset/ingest.py
python archive/caddieset/study_panel.py   # needs numpy and scikit-learn: archive/README.md
```
