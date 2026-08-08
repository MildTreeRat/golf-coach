# AI Golf Swing Trainer

A home-lab AI-powered golf swing analysis system that captures your swing via camera, integrates launch monitor data, analyzes mechanics, and delivers coaching feedback.

## Project Status

**Working today, no hardware required:** drop a face-on swing clip in `data/raw/`, and the
pipeline extracts pose, segments the swing, scores three checkpoints against tour-derived
benchmark bands, and prints ranked coaching tips with an annotated verification overlay.
Shot data is read off photographs of the HD Golf simulator screen by local OCR.

| Milestone | State |
|---|---|
| **M1** Capture & skeleton | ✅ Done — MediaPipe pose, face-on canonical angle |
| **M1.5** Club-head detectability spike | ⬜ Not started — gates M2 |
| **M2** Club & ball detection (YOLOv8) | 🔒 Gated on M1.5 + global-shutter camera |
| **M3** Launch monitor / MCP | 🟡 Shot ingestion done (screen OCR); MCP server pending |
| **M4-PoC / PoC+ / REF** Pose-only analysis | ✅ Done — 3 checkpoints, bands validated vs 461 tour clips |
| **M5-FB** Prioritised coaching feedback | ✅ Done — ranked tips, tour percentiles |
| **M4** full (outcome axis) | ⬜ Needs the M2 + M3 streams |
| **M5** Feedback UI · **M6** LLM coaching | ⬜ Not started |
| **M7** Two-phone sim capture | 🟡 4/7 phases (1, 2, 3, 5) |

See **[docs/README.md](docs/README.md)** for the documentation map,
[ROADMAP.md](ROADMAP.md) for milestone detail, and [WORKLOG.md](WORKLOG.md) for
session-by-session notes (the best "pick up where I left off" document).

## Getting Started

```bash
python -m venv .venv && source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -e '.[dev]'          # base + dev tools — the whole analysis core runs on this
pytest                           # analysis, contracts, feedback and launch-monitor suites
```

The four working CLIs:

```bash
# 1. Pose: video -> keypoints JSON + skeleton overlay          (needs the `vision` extra)
pip install -e '.[vision,dev]'
python scripts/run_pose.py data/raw/my_swing.mov --camera-id face_on
#    -> data/processed/my_swing.keypoints.json   (+ fps/size/hash, and which camera)
#    -> data/processed/my_swing.overlay.mp4
#    --camera-id is optional; it matters once two angles of one swing are in play (ADR-011)

# 2. Analysis: keypoints -> scores, ranked tips, detected instants     (base install)
python scripts/analyze_swing.py data/processed/my_swing.keypoints.json
#    add --overlay to render ADDRESS/TOP/IMPACT markers + a score HUD (needs `vision`)
python scripts/analyze_swing.py data/processed/my_swing.keypoints.json \
    --overlay data/raw/my_swing.mov

# 3. Alignment: two angles of ONE swing -> a side-by-side video, synced      (base install;
#    `vision` only to render)
python scripts/align_swings.py face_on.keypoints.json down_the_line.keypoints.json \
    --video-a face_on.MOV --video-b down_the_line.MOV --out aligned.mp4
#    the two clips are aligned on the swing's own instants, never on a clock, so mismatched
#    frame rates / clip lengths / start moments and iPhone slo-mo all cancel (ADR-015)
#    --list-swings first if a clip contains practice swings; --window-b 1800:2400 to pick one

# 4. Shot data: photos of the simulator SHOT DATA screen -> parsed shots  (needs `ocr`)
pip install -e '.[ocr]'
python scripts/import_shot_screens.py data/raw/shot_screens --dry-run
```

Reading the parsed shots back needs no extras at all — `ScreenShotDataSource` serves them
from the store on the base install.

### Uploading swings from a phone

The upload server always binds `127.0.0.1`. Phones reach it through Tailscale, which
terminates TLS and proxies inward — the bind never widens (ADR-016).

```bash
pip install -e '.[api]'
echo "GOLF_UPLOAD_TOKEN=$(python -c 'import secrets; print(secrets.token_urlsafe(24))')" >> .env
python scripts/run_server.py          # http://127.0.0.1:3000/
```

Then, on the desktop, put Tailscale in front of it:

```bash
tailscale serve  --bg 3000    # your own devices, over the tailnet — direct, unmetered
tailscale funnel --bg 3000    # anyone with the link (a helper's phone) — relayed
tailscale funnel --bg off     # turn guest access back off when the session ends
```

Both publish `https://<machine>.<tailnet>.ts.net`. Open **`https://<machine>.<tailnet>.ts.net/?t=<token>`**
once per phone: the page stores the token and the role, strips the token from the URL, and every
upload after that is two taps. Verify it properly with WiFi *off* — on cellular is the real test.

Notes: Funnel needs a `funnel` node attribute in the tailnet policy file, only listens on
443/8443/10000, and is relayed with undisclosed bandwidth limits — prefer Serve for your own
phones. Without `GOLF_UPLOAD_TOKEN` set, `/api/` is unauthenticated, which is only acceptable
tailnet-only; `run_server.py` refuses a non-loopback `--host` in that state.

The default port is 3000 rather than 8080 because Windows reserves 8069–8168 (and 7969–8068,
8169–8268) on this machine, so 8080/8000/8443 fail to bind with `WinError 10013`. Check with
`netsh interface ipv4 show excludedportrange protocol=tcp`.

Offline research tooling for the reference corpus lives in `scripts/golfdb/` and needs the
`research` extra; see [data/README.md](data/README.md) for how to rebuild it.

## Architecture

Nine packages under `src/golf_coach/`, with a shared `contracts/` package as the seam every
module depends on — modules never import each other.

- **contracts** — shared Pydantic data shapes; the decoupling seam (ADR-008)
- **capture** — `VideoSource` port; `FileVideoSource` adapter over OpenCV
- **pose** — MediaPipe Tasks API → `FrameKeypoints` (33 landmarks/frame), plus overlay rendering
- **detection** — YOLOv8 club head + ball *(stub — M2, gated on the M1.5 spike)*
- **launch_monitor** — `ShotDataSource` port with mock / screen-OCR / composite adapters
- **analysis** — pure functional core: smooth → phases → checkpoints → score, plus
  two-view alignment on a normalized swing-time axis (ADR-015)
- **feedback** — rule-based ranked tips; Claude coaching and overlays to come
- **storage** — flat-file, content-addressed swing-bundle store *(M7 Phase 3, trimmed)*
- **api** — FastAPI phone-upload server *(M7 Phase 5, trimmed — ingestion only, no analysis wiring yet)*

[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) documents the system **as built**;
[docs/FLOW.md](docs/FLOW.md) documents the **target** design and build order.

## Tech Stack

| Layer | Technology | Status |
|-------|-----------|--------|
| Language | Python 3.11+ (backend/ML), JavaScript/React (UI) | Python in use |
| Pose estimation | MediaPipe Pose Landmarker, **lite** variant (Tasks API) | In use |
| Video processing | OpenCV | In use |
| Reference data | GolfDB — 1,399 hand-annotated tour swings (ADR-012) | In use, aggregates only |
| Shot data OCR | PaddleOCR, reading the HD Golf screen (ADR-014) | In use |
| Object detection | YOLOv8 (Ultralytics) | M2, not started |
| Backend API | FastAPI | In use — phone upload server, loopback + Tailscale (ADR-016) |
| MCP server | Python (MCP SDK) | M3, not started |
| Database | SQLite | Reserved, unused — storage is flat-file (see `storage/`) |
| LLM | Claude API (Anthropic) | M6, not started |
| Frontend | React | M5, not started |

Heavy dependencies are optional extras, so the analysis core installs and tests with none of
them: `vision`, `api`, `llm`, `hardware`, `ocr`, `research`, `dev`, and `all`. See
`pyproject.toml`.

## Project Structure

The layout and the reasoning behind it are documented in [ADR-008](docs/decisions/008-project-structure.md).
The core idea: a shared `contracts/` package is the seam every module depends on (modules
never import each other), I/O boundaries use swappable real/mock adapters, and the analysis
engine is a pure functional core. This is what lets sections be built independently and run
on simulated data before any hardware exists.

```
golf-coach/
├── README.md  ROADMAP.md  WORKLOG.md
├── pyproject.toml           # deps (base + 7 extras) + tooling
├── docs/
│   ├── README.md            # ⭐ documentation map — start here
│   ├── ARCHITECTURE.md      # the system as built
│   ├── FLOW.md              # the target design + build order
│   ├── PROJECT_CHARTER.md
│   ├── decisions/           # ADRs 000–014
│   └── archive/             # superseded milestone docs, kept as record
├── src/
│   └── golf_coach/
│       ├── contracts/       # ⭐ shared data shapes (Pydantic) — the decoupling seam
│       ├── capture/         # VideoSource port + file adapter
│       ├── pose/            # MediaPipe → FrameKeypoints + overlays
│       ├── detection/       # YOLOv8 → FrameDetections (stub, M2)
│       ├── launch_monitor/  # ShotDataSource port + mock/screen/composite adapters
│       ├── analysis/        # ⭐ pure functional core: smooth→phases→checkpoints→score
│       ├── feedback/        # ranked rule-based tips (+ Claude coaching later)
│       ├── storage/         # flat-file swing-bundle store, content-addressed
│       ├── api/             # FastAPI phone-upload server (ingestion only, no analysis yet)
│       └── config.py        # settings (the only env reader)
├── frontend/                # React UI (M5) — separate toolchain, talks to api/ over HTTP
├── tests/                   # mirrors the package; the core suite runs on the base install
├── spikes/                  # throwaway exploration (e.g. the M1.5 detectability spike)
├── scripts/                 # dev CLIs + scripts/golfdb/ reference-data tooling
└── data/                    # gitignored: raw/ processed/ (incl. sessions/) models/ reference/
```

## Decision Log

All architectural and technology decisions are documented as ADRs in `docs/decisions/` —
16 of them, several carrying dated addenda where reality corrected the original call.
[docs/README.md](docs/README.md) indexes them with statuses; see
[000-template.md](docs/decisions/000-template.md) for the format.
