# ADR-004: Launch Monitor Hardware

## Status
Accepted

## Date
2026-03-16

## Context
Need a consumer launch monitor that provides both club metrics (face angle, club path, club head speed) and ball metrics (ball speed, launch angle, spin rate, spin axis, smash factor). Club data is essential — the swing analysis engine needs to correlate what the camera sees (body mechanics) with what the club actually did at impact.

The deciding factor is not raw accuracy but **programmatic data extraction** — how easily can we get real-time shot data into our pipeline without manual steps.

## Options Considered

### Option A: Garmin Approach R10 (~$500 new, ~$300-400 used)
- **Pros**: Measures full club data (face angle, club path, club head speed) AND ball data (ball speed, launch angle, spin rate, spin axis, smash factor). Bluetooth Low Energy (BLE). Largest community reverse-engineering effort of any consumer monitor. Multiple open-source projects extract real-time shot data:
  - `gsp-r10-adapter` (mholow) — direct BLE connection to PC, intercepts shot data in real time
  - `gspro-garmin-connect-v2` (travislang) — intercepts shot data via the Garmin app's E6 Connect protocol
  - Garmin Connect bulk data export — JSON files with full session history
  - Active Web Bluetooth API experiments for browser-based real-time data
- **Cons**: Accuracy debated for some metrics (especially spin — uses algorithmic estimation, not direct measurement). Indoor use requires ~6 feet of ball flight. Radar-based, so needs line of sight behind the ball.

### Option B: Rapsodo MLM2PRO (~$500)
- **Pros**: Camera + radar hybrid. Good accuracy. Compact.
- **Cons**: Data locked in Rapsodo app. No known open-source data extraction tools. Would require reverse engineering from scratch with no community support. Dead end for programmatic integration.

### Option C: Square Golf Launch Monitor (~$700-900)
- **Pros**: Sub-$1,000 photometric. GSPro compatible via Open API. Emerging player.
- **Cons**: Newer device, less community tooling. Indoor only (original model). Limited club data on base model. Less proven hackability.

### Option D: FlightScope Mevo Gen 2 (~$2,000)
- **Pros**: Excellent accuracy. Radar-based. USB-C. Some API/export options.
- **Cons**: 4x the price of R10 for marginally better accuracy. Over budget for a learning project.

### Option E: PiTrac — Open Source DIY Launch Monitor (~$250-400 to build)
- **Pros**: Fully open source. Maximum learning. Uses Raspberry Pi + cameras + IR strobes. Measures ball speed, launch angles, spin in 3 axes. ~$250 in parts.
- **Cons**: Ball data ONLY — no club metrics (face angle, club path, club speed). Requires soldering, 3D printing, Linux configuration. Still immature — "half the features barely work." A significant sub-project that would distract from the primary AI/software goals.

### Option F: Simulated/mock data (no hardware)
- **Pros**: Free. No hardware dependency. Can start immediately.
- **Cons**: No real data means no real validation. Delays the feedback loop of actually hitting balls and seeing results.

## Decision
**Garmin Approach R10** — buy used if possible (~$300-400).

The R10 wins decisively on data accessibility. It is the only consumer launch monitor under $1,000 with multiple proven, open-source tools for real-time programmatic data extraction via Bluetooth. The BLE protocol has been reverse-engineered by the community, and working Python/C# code exists to intercept shot data on a per-shot basis.

It provides all required club metrics (face angle, club path, club head speed) plus comprehensive ball data. While its spin accuracy is debated compared to photometric systems, for the purpose of this project — correlating club data with AI-analyzed swing mechanics — the R10's data is more than sufficient.

The device is truly plug-and-play: power it on, pair via Bluetooth, and it captures data autonomously. No complex setup, calibration rigs, or ongoing maintenance.

## Data Extraction Strategy

Three approaches, in order of priority:

### 1. Real-Time BLE (Primary — for MCP server)
Use the `gsp-r10-adapter` approach (or write our own Python BLE client using `bleak` library) to:
- Pair with R10 via Bluetooth
- Listen for shot notifications
- Parse shot data into our `ShotData` schema
- Push to MCP server immediately after each shot

Reference repos:
- https://github.com/mholow/gsp-r10-adapter
- https://github.com/travislang/gspro-garmin-connect-v2

### 2. Garmin App E6 Protocol (Fallback)
The R10's Garmin Golf app has an "E6 Connect" mode that broadcasts shot data over the local network. The `gspro-garmin-connect-v2` tool intercepts this data. This is easier to set up but requires the Garmin app running on a phone.

### 3. Bulk Export (Historical analysis)
Request data export from Garmin Connect account → JSON files → parse with existing community scripts → import into SQLite for session comparison and trend analysis.

## Shot Data Schema (from R10)

| Metric | Unit | Description |
|--------|------|-------------|
| club_head_speed | mph | Speed of club head at impact |
| ball_speed | mph | Speed of ball off the face |
| launch_angle | degrees | Vertical launch angle |
| launch_direction | degrees | Horizontal launch direction (+ = right) |
| spin_rate | rpm | Total spin rate |
| spin_axis | degrees | Spin axis tilt (+ = right/fade) |
| club_face_angle | degrees | Face angle at impact relative to target (+ = open) |
| club_path | degrees | Club path relative to target (+ = in-to-out) |
| smash_factor | ratio | Ball speed / club head speed |
| carry_distance | yards | Estimated carry distance |
| total_distance | yards | Estimated total distance |
| apex_height | yards | Max height of ball flight |

## Shopping List Addition

| Item | Qty | Est. Price | Notes |
|------|-----|-----------|-------|
| Garmin Approach R10 (used) | 1 | ~$300-400 | Check eBay, Facebook Marketplace, r/golfsimulator |
| Tripod/mount for R10 | 1 | ~$15-25 | R10 has standard tripod mount. Position 6-8 feet behind ball, aligned with target. |
| **Subtotal** | | **~$315-425** | |

### Updated Total Project Hardware Budget

| Item | Est. Price |
|------|-----------|
| 2x ELP AR0234 cameras + tripods + cables | ~$240 |
| Garmin Approach R10 (used) + mount | ~$350 |
| **Total** | **~$590** |

## Consequences
- Real-time shot data extraction is a solved problem thanks to community work. Our MCP server can receive shot data within seconds of each swing.
- Club metrics (face angle, path, speed) enable direct correlation with camera-observed swing mechanics — e.g., "your hands were in X position at impact AND the face was 3° open."
- The `bleak` Python BLE library can connect to the R10 directly, keeping the entire data pipeline in Python.
- R10 accuracy limitations on spin are acceptable — we're using launch monitor data to ground-truth our pose analysis, not to replace a Trackman.
- If R10 accuracy proves insufficient later, swapping to a different launch monitor only changes the MCP server's data parser — the tool interface and analysis engine remain unchanged (low coupling in action).
- Can start with simulated data in Milestones 1-2 and integrate the R10 in Milestone 3 as planned.
