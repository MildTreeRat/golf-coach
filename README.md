# AI Golf Swing Trainer

A home-lab AI-powered golf swing analysis system that captures your swing via camera, integrates launch monitor data, analyzes mechanics, and delivers coaching feedback.

## Project Status

> **⭐ NEXT ACTION — do this first:** add a real `GOLF_ANTHROPIC_API_KEY` to `.env` and run
> `python scripts/analyze_bundle.py 2026-08-10/2 --no-video` to verify M6 coaching against the
> live API. The coaching path is built and green, but only against a fake client — no real request
> has been sent yet. See the **NEXT ACTION** note atop [ROADMAP.md](ROADMAP.md) for what to check
> in the output.

**Working today, no hardware required:** drop a face-on swing clip in `data/raw/`, and the
pipeline extracts pose, segments the swing, scores six checkpoints against tour-derived
benchmark bands, and prints ranked coaching tips with an annotated verification overlay.
Shot data is read off photographs of the HD Golf simulator screen by local OCR. Point
`analyze_bundle.py` at a swing uploaded from two phones and it does the lot in one command,
leaving a JSON result and an aligned side-by-side video beside the clips.

**From a phone, it needs no command at all:** upload a face-on clip, a down-the-line clip and a
photo of the shot screen from two phone browsers, and the third file triggers the same pipeline
in the background — the results page has the score, the bands, the tips and the aligned video by
the time you have walked back from the bay.

| Milestone | State |
|---|---|
| **M1** Capture & skeleton | ✅ Done — MediaPipe pose, face-on canonical angle |
| **M1.5** Club-head detectability spike | ⬜ Not started — gates M2 |
| **M2** Club & ball detection (YOLOv8) | 🔒 Gated on M1.5 + global-shutter camera |
| **M3** Launch monitor / MCP | 🟡 Shot ingestion (screen OCR) + MCP server done; OCR tuning left |
| **M4-PoC / PoC+ / REF** Pose-only analysis | ✅ Done — 6 checkpoints, bands validated vs 461 tour clips |
| **M5-FB** Prioritised coaching feedback | ✅ Done — ranked tips, tour percentiles |
| **M4** full (outcome axis) | ⬜ Needs the M2 + M3 streams |
| **M6** LLM coaching | 🟡 Claude writes the per-swing verdict; follow-up Q&A left |
| **M6.5** Measure now, judge later | ✅ Done — 9 metrics recorded per swing, 6 of them scored |
| **Career mode** One golfer over time | ✅ 6/6 steps — built, and currently **silent by design**: it reports **n = 2** per metric and refuses every claim over that. A bay session is what makes it speak |
| **M5** Feedback UI | ⬜ Not started — a static results page stands in for it |
| **M7** Two-phone sim capture | 🟡 6/7 phases — only the Phase 0 field spike is left |

See **[docs/README.md](docs/README.md)** for the documentation map,
[ROADMAP.md](ROADMAP.md) for milestone detail, and [WORKLOG.md](WORKLOG.md) for
session-by-session notes (the best "pick up where I left off" document).

## Getting Started

```bash
python -m venv .venv && source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -e '.[dev]'          # base + dev tools — the whole analysis core runs on this
pytest                           # analysis, contracts, feedback and launch-monitor suites
```

The five working CLIs:

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

# 5. The whole thing: an assembled swing bundle -> the complete result       (`vision`; `ocr`
#    only for a shot photo that isn't in the store yet)
python scripts/analyze_bundle.py 2026-08-07/1        # SESSION/SWING, or a swing directory
#    -> analysis.json   score, checkpoints, ranked tips, the HD Golf numbers, alignment quality
#    -> aligned.mp4     the two views side by side, banners landing together
#    -> <role>.keypoints.json   pose per view, cached by the clip's hash so a re-run is free
#    picks the real swing out of a clip full of practice swings by downswing duration;
#    --list-swings to see the candidates, --window-face-on / --window-dtl to override

# 6. One golfer across every session: the honest sample size            (base install)
python scripts/career_corpus.py --name Aaron
#    -> distinct swings after collapsing re-uploads, and n per metric
#    swings are deduped on the face-on clip's hash, launch-monitor metrics on the shot
#    photo's, so a clip uploaded three times counts once — see "Career mode" in ROADMAP.md
python scripts/backfill_golfer.py --name Aaron --handedness right   # label older swings
python scripts/reanalyze.py --dry-run                # which stored results are out of date
#    a swing is out of date when it was never analyzed, when a clip was re-uploaded since, or
#    when it was analyzed by an older engine (`analysis_version`). Drop --dry-run to repair;
#    pose and shots are cached, so an unchanged bundle re-runs in seconds

# 7. What that sample size buys — which today is nothing, on purpose      (base install)
python scripts/career_baseline.py --name Aaron
#    -> per metric: center, spread, trend, and where the center sits against the tour band
python scripts/career_dispersion.py --name Aaron
#    -> is the miss repeatable (look at what is fixed before the swing) or scattered (timing)?
#    Both refuse every claim at n = 2, and each refusal names what it waits for — so the output
#    doubles as a worklist. A baseline printed over two swings is the failure this exists to
#    avoid. Same data on a page at /career.html?player=aaron, and over MCP as
#    get_golfer_profile / get_shot_trends / compare_sessions.
#    Windows: prefix with PYTHONIOENCODING=utf-8, or the console garbles the em-dashes
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

**Taking this to an actual sim bay?** [docs/BAY_SESSION_RUNBOOK.md](docs/BAY_SESSION_RUNBOOK.md)
is the preflight checklist, the phone camera settings (1080p60, not 4K, and why), the measured
timings, and what to check when it doesn't work.

**The third file starts the analysis.** When a swing has all three roles, a background worker
runs the same pipeline `analyze_bundle.py` does — pose per view, the shot screen, scoring,
alignment — and the swing gets a **View results** link on the upload page. Nothing auto-runs on
a partial bundle: no timeout guesses right about whether the second phone is still walking back
from the bay. Shot the swing from one angle on purpose? **Analyze anyway** on that swing runs it
without the missing pieces, after warning you the result will be thinner. A face-on clip is the
one hard requirement — every checkpoint is measured from it.

Uploads never wait on analysis: the pipeline runs in a thread, so you can be uploading the next
swing while the last one is still being posed (~30 s for a 30 fps pair, minutes at 4K60).

The results page shows the score, each checkpoint against its tour band and percentile, the
ranked tips, Claude's written verdict if a key is configured, the HD Golf numbers, and the two
views side by side. Run the server with `python scripts/run_server.py` and watch the terminal —
the worker narrates every step.

> **OpenH264 warnings on every render are expected and harmless.** OpenCV's bundled FFmpeg ships
> no libx264, only libopenh264, and dlopen's a DLL that isn't included — so it prints
> `Failed to load OpenH264 library` and gives up. OpenCV then falls back to Media Foundation,
> which encodes genuine H.264, which is what makes `aligned.mp4` playable in the phone browser.
> Dropping [openh264-1.8.0-win64.dll](https://github.com/cisco/openh264/releases) on the DLL
> search path silences the noise; nothing else changes. If both paths ever fail the renderer
> falls back to `mp4v`, records that it did, and the results page says the clip needs VLC.

Notes: Funnel needs a `funnel` node attribute in the tailnet policy file, only listens on
443/8443/10000, and is relayed with undisclosed bandwidth limits — prefer Serve for your own
phones. Without `GOLF_UPLOAD_TOKEN` set, `/api/` is unauthenticated, which is only acceptable
tailnet-only; `run_server.py` refuses a non-loopback `--host` in that state.

The default port is 3000 rather than 8080 because Windows reserves 8069–8168 (and 7969–8068,
8169–8268) on this machine, so 8080/8000/8443 fail to bind with `WinError 10013`. Check with
`netsh interface ipv4 show excludedportrange protocol=tcp`.

### Claude coaching (M6)

With an Anthropic API key configured, every analyzed swing also gets a short written verdict —
the paragraph a coach gives walking back from the bay, written from the same numbers the page
shows and nothing else.

```bash
pip install -e '.[llm]'
echo "GOLF_ANTHROPIC_API_KEY=sk-ant-..." >> .env
```

That is the whole setup. With no key the call is skipped, the swing still scores, and the results
page says why in its notes — coaching is the last thing that happens to a result and is never
allowed to cost you one. `--no-coaching` turns it off for a run even with a key set.

The prose is always attributed to the model that wrote it, on the page and in `analysis.json`
(`feedback.coaching`). That is not decoration: everything else on that page is measured, and a
reader has to be able to tell which is which. The prompt carries the same standing caveats the
MCP server shows a client — they live in one place, `contracts/caveats.py`, because two copies
of load-bearing prose is one copy going stale.

### Asking Claude about your swings (MCP)

`scripts/run_mcp_server.py` speaks stdio, which is what Claude Desktop and Claude Code both use.
Register it once and you can ask about sessions in plain language.

```bash
# Claude Code, from the repo root
claude mcp add golf-coach -- "$(pwd)/.venv/Scripts/python.exe" "$(pwd)/scripts/run_mcp_server.py"
```

For **Claude Desktop**, add this to `%APPDATA%\Claude\claude_desktop_config.json` (absolute paths,
doubled backslashes) and restart it:

```json
{ "mcpServers": { "golf-coach": {
    "command": "C:\\path\\to\\golf-coach\\.venv\\Scripts\\python.exe",
    "args": ["C:\\path\\to\\golf-coach\\scripts\\run_mcp_server.py"] } } }
```

Then ask *"how did my swing on 2026-08-10 go?"* — five tools cover sessions, one swing's full
analysis, a session summary, and recent shots.

Offline research tooling for the reference corpus lives in `scripts/golfdb/` and needs the
`research` extra; see [data/README.md](data/README.md) for how to rebuild it.

## Architecture

Nine packages under `src/golf_coach/`, with a shared `contracts/` package as the seam every
module depends on — modules never import each other.

- **contracts** — shared Pydantic data shapes; the decoupling seam (ADR-008)
- **capture** — `VideoSource` port; `FileVideoSource` adapter over OpenCV
- **pose** — MediaPipe Tasks API → `FrameKeypoints` (33 landmarks/frame), plus overlay and
  side-by-side rendering
- **detection** — YOLOv8 club head + ball *(stub — M2, gated on the M1.5 spike)*
- **launch_monitor** — `ShotDataSource` port with mock / screen-OCR / composite adapters
- **analysis** — pure functional core, with **measuring split from judging** (M6.5): `measure.py`
  and `shot_measure.py` produce numbers with no band in sight, `checkpoints/` turns three of them
  into verdicts. The split is what lets a new metric be measured across the reference corpus
  *before* a band for it exists — bands are derived from those measurements, so the fused version
  could never let a new metric acquire one. Also: smooth → phases → checkpoints → score, plus
  two-view alignment on a normalized swing-time axis (ADR-015) and whole-bundle analysis
  (`analyze_swing_bundle`: face-on scored, down-the-line for anchors, shot attached)
- **feedback** — rule-based ranked tips (`rules.py`), plus the Claude coaching paragraph
  (`coach.py`, M6) written from those tips and the measured numbers, and stamped with the model
  that wrote it so prose is never mistaken for a measurement
- **storage** — flat-file, content-addressed swing-bundle store *(M7 Phase 3, trimmed)*;
  analysis artifacts land in the same swing directory
- **api** — FastAPI phone-upload server, the bundle pipeline, and the background worker that
  runs it when a swing completes *(M7 Phase 5)*

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
| MCP server | Python (MCP SDK) | In use — swings + shots over stdio (`scripts/run_mcp_server.py`) |
| Database | SQLite | Reserved, unused — storage is flat-file (see `storage/`) |
| LLM | Claude API (Anthropic), `claude-opus-5` | In use — per-swing coaching (M6) |
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
│   ├── decisions/           # ADRs 000–016
│   └── archive/             # superseded milestone docs, kept as record
├── src/
│   └── golf_coach/
│       ├── contracts/       # ⭐ shared data shapes (Pydantic) — the decoupling seam
│       ├── capture/         # VideoSource port + file adapter
│       ├── pose/            # MediaPipe → FrameKeypoints + overlays
│       ├── detection/       # YOLOv8 → FrameDetections (stub, M2)
│       ├── launch_monitor/  # ShotDataSource port + mock/screen/composite adapters
│       ├── analysis/        # ⭐ pure functional core: smooth→phases→checkpoints→score,
│       │                    #   plus career mode: baseline → dispersion → tour comparison
│       ├── feedback/        # ranked rule-based tips (+ Claude coaching later)
│       ├── storage/         # flat-file stores: swing bundles (content-addressed),
│       │                    #   golfer registry, career corpus reader
│       ├── api/             # upload server + bundle pipeline + background analysis worker
│       ├── mcp/             # MCP tools over swings, shots and one golfer's career (ADR-006)
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
