# Bay Session Runbook — taking the two-phone capture to a real sim

**Tier**: AS-BUILT — every command below runs against what is in the repo today.
**Status**: written 2026-08-09, before the first bay session.
**Decisions**: [ADR-016](decisions/016-local-first-host-and-phone-upload-topology.md) (how a phone
reaches the desktop), [ADR-015](decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md)
(why hand-held phones are aligned, never fused), [ADR-014](decisions/014-screen-capture-shot-ingestion.md)
(the offline-first stance this whole topology serves).

> ⚠️ **The failure-mode table below is predicted, not observed.** Every row traces to a real
> mechanism in the code or a measured number, but no bay session has happened yet — so nothing
> here is a record of a fault that actually occurred. After the first session, fold what really
> broke back into §6 and drop this banner. Per the conventions in
> [README.md](README.md#conventions): a decision gets an ADR, a measurement gets a findings
> section, and a stale number gets a banner rather than a silent edit.

M7 built the whole path — two phones upload, a worker analyses, a results page renders — and it
has been verified from a phone on cellular ([WORKLOG 2026-08-06](../WORKLOG.md)). What it has
never had is a session at an actual bay. This page is what to do before driving out, what to do
while there, and what to check when something doesn't work.

---

## 1. At-home preflight

Do all of this **at home, the day before or the morning of**. Every one of these failures is
cheap here and expensive at the bay.

```bash
pip install -e '.[api,vision,ocr]'     # api serves, vision poses, ocr reads the shot screen
cat .env                               # must contain GOLF_UPLOAD_TOKEN=...
python scripts/run_server.py           # prints the phone setup link — leave this running
```

In a second terminal:

```bash
tailscale serve --bg 3000
tailscale serve status                 # confirms https://<machine>.<tailnet>.ts.net → 127.0.0.1:3000
```

Then, checklist:

- [ ] **Both phones set up once.** Open `https://<machine>.<tailnet>.ts.net/?t=<token>` on each,
      pick the role (`face-on` on one, `down-the-line` on the other). The page stores the token
      and the role in `localStorage` and strips the token from the URL — after this, uploading is
      two taps.
- [ ] **The golfer is set.** The bar at the top of the upload page. Type the name, pick
      right- or left-handed the first time that name is used, tap Save. Either phone can do it —
      it is one value for the session and both phones read it from the server, so it is set once,
      not once per phone.

      This is the only thing you record at the bay that **cannot be recovered afterwards**.
      Everything else is derived from the clips and can be recomputed whenever the code improves;
      who swung and which way they face are not in the footage in any form a machine can read.
      Handedness in particular is the frame of reference for every signed measurement, so a wrong
      one does not fail loudly, it silently inverts them. Career mode is per-golfer by
      construction, so a session captured without this is a session it cannot use.

      Uploads are deliberately **not blocked** when it is unset — you would lose swings to a form.
      The page shows an orange "No golfer set" banner instead, and setting the golfer adopts every
      swing already uploaded that is still unlabeled. Handing the club to someone else mid-session
      is fine: change the name, and only swings from that point carry it. If a swing ends up
      under the wrong name, each swing row has a **change** link.
- [ ] **One real clip uploaded end to end, with the phone's WiFi OFF.** Cellular is the real
      test; on home WiFi a broken tailnet path can still succeed by accident. Confirm a **View
      results** link appears and the results page renders.
- [ ] **The desktop will not sleep.** This is the failure that is invisible from the bay — a
      sleeping machine takes its tailnet node down with it.
      ```powershell
      powercfg /change standby-timeout-ac 0
      powercfg /change hibernate-timeout-ac 0
      ```
- [ ] **`run_server.py` is actually running**, and will still be running when you get there.
      `tailscale serve --bg` persists across a reboot; **uvicorn does not.** A live serve config
      in front of a dead server answers with a 502 that looks exactly like a network fault. There
      is no service wrapper or autostart today — no `[project.scripts]`, no Makefile — so this is
      a manual check. (Known gap; not built here.)
- [ ] **Disk headroom** in `data/processed/sessions/`. Budget ~100 MB per swing for two 1080p60
      clips plus the shot photo, before the aligned render.

### Phone camera settings — 1080p60, not 4K

Set both phones to **1080p60** in Settings → Camera → Record Video. This matters more than
anything else on this page:

- **Upload is the dominant latency in the whole system, not compute** (ADR-016). Each clip
  crosses the sim's uplink, which is usually the worst leg of the path.
- **4K60 buys nothing analytically.** Scoring runs on the three validated face-on 2D checkpoints
  and MediaPipe downscales its input anyway; down-the-line is capture-and-align only (ADR-015).
  You would pay roughly 4× the bytes for pixels nothing reads.
- **4K60 pushes analysis from ~30 s into minutes**, so the results page stops being ready by the
  time you walk back from the bay — which is the entire point of the design.

Record normal video, not slo-mo, unless you are deliberately collecting Phase 0's Q3 data — slo-mo
stores 120/240 fps behind a stretched playback rate, and `CAP_PROP_FPS` may not describe real
time. That is an open question, not a settled one.

---

## 2. Guest-phone preflight (Funnel) — only if a helper is filming

The down-the-line phone often belongs to whoever is at the bay, and **a helper cannot join your
tailnet**. That is what Funnel is for, and it is the path that makes `GOLF_UPLOAD_TOKEN`
mandatory rather than optional — Funnel publishes the upload endpoint to the public internet.

```bash
tailscale funnel --bg 3000     # needs the `funnel` node attribute in the tailnet policy file
```

Send the helper `https://<machine>.<tailnet>.ts.net/?t=<token>`. They pick `down-the-line` once
and are done — no app, no account, no install.

**Turn it off when the session ends:**

```bash
tailscale funnel --bg off
```

Leaving Funnel on is the single riskiest state this design permits (ADR-016). Make it the last
line of the session, not a thing you remember on the drive home.

> ⬜ **Validate this before you rely on it — it has never been exercised end-to-end.** Serve was
> verified with the token on, but tailnet membership was also protecting the endpoint, so the
> token has never actually been the only thing holding the door. Funnel is where it is.
>
> Run this once, at home, from a phone that is **not** on the tailnet (Tailscale app logged out,
> WiFi off):
>
> 1. `tailscale funnel --bg 3000` — confirm it reports a public URL rather than an error about
>    the `funnel` node attribute.
> 2. Open `https://<machine>.<tailnet>.ts.net/` — the page should load with a trusted certificate.
> 3. Hit `/api/sessions/current` with no token — must be **401**.
> 4. Open `/?t=<token>` and upload one real 1080p60 clip — must be **200**. **Time it**, and put
>    the number in the blank row in §3.
> 5. `tailscale funnel --bg off` — confirm the public URL stops serving.
>
> A failure here is a finding, not a blocker for the bay: fall back to Serve for your own phones
> and AirDrop for the helper's.

**If Funnel is too slow at the bay**, the fallback is the one ADR-016 already names: the helper
AirDrops the clip to your phone and you upload it over Serve. Degraded, not broken.

---

## 3. Timings — what to actually expect

Measured, not estimated, except where marked.

| Leg | Number | Source |
|---|---|---|
| 1080p60, ~10 s clip | 30–50 MB; **33.6 MB** observed face-on | ADR-016; WORKLOG 2026-08-06 |
| Three-file bundle, Serve, on cellular | **30 s** | WORKLOG 2026-08-06 |
| Analysis, 30 fps pair | **31.8 s** — and uploads never wait on it | WORKLOG 2026-08-09 |
| Analysis, 4K60 | minutes | README |
| Three-file bundle, **Funnel** (guest) | **unmeasured** — relayed, undisclosed bandwidth limits | — |

The shape to expect: uploads dominate, analysis overlaps them because the pipeline runs in a
thread, and the results page is ready roughly by the time you have walked back from the bay.

---

## 4. The per-swing loop at the bay

0. **Glance at the golfer bar.** Green name = filed correctly. Orange banner = the swings you are
   about to hit are landing unlabeled; fix it whenever you get a moment, and they will be adopted
   retroactively. Only needed once per session unless someone else takes a turn.
1. Hit the shot.
2. Face-on phone: upload the clip (role already remembered — pick the file, tap upload).
3. Down-the-line phone: same.
4. Photograph the HD Golf `SHOT DATA` screen and upload it as `shot-screen`.
5. **The third file starts the analysis.** A **View results** link appears on the upload page
   when it finishes.

Nothing auto-runs on a partial bundle — no timeout guesses right about whether the second phone
is still walking back from the bay. Shot from one angle on purpose? Use **Analyze anyway** on
that swing; it warns you the result will be thinner. **A face-on clip is the one hard
requirement** — every checkpoint is measured from it.

You do not have to wait for a swing to finish before recording the next one.

---

## 5. Two things that will look like bugs and are not

- **`Failed to load OpenH264 library` on every render.** Expected and harmless. OpenCV falls back
  to Media Foundation, which encodes real H.264 — which is what makes the aligned video playable
  in the phone browser. See README.
- **Tempo is trustworthy again as of 2026-08-09.** It used to read around 0.4:1 on real footage —
  a backswing shorter than its own downswing, which is physically impossible — and the ranked tips
  led with it. The cause was not tempo at all: a hover at the top fragmented the descent and put
  the detected *top* ten frames late, so the downswing was measured at 14 frames when it was 24.
  Fixed in `phases._DRAWDOWN_FLOOR`. If you see a tempo below 1:1 again, that is the same class of
  bug returning and the number should be ignored — the phase boundaries are wrong, not your swing.
- **"Aligned on top and impact" under the video is normal, not an error.** Only the top tier
  ("aligned on motion start, top and impact") means all three instants were agreed by both
  cameras. The takeaway is the hardest instant to locate and is often estimated instead; the note
  under the video says which camera and why. The swing itself is still aligned on the two anchors
  that matter most.

---

## 6. When it doesn't work

| Symptom | Most likely cause | What to check |
|---|---|---|
| Page won't load at all | Desktop asleep, or `run_server.py` not running | Tailscale app on the phone shows the node's state. A **502** specifically means the tailnet path is fine and uvicorn is dead — `tailscale serve --bg` survived a reboot that uvicorn didn't. |
| Certificate warning | MagicDNS / HTTPS certificates not enabled for the tailnet | Enable both in the Tailscale admin console, then re-run `tailscale serve --bg 3000` |
| Page loads, upload returns **401** | The page strips `?t=` from the URL after first load, so a phone that cleared its `localStorage` has no token | Re-open the full `/?t=<token>` setup link. Also check the token wasn't rotated in `.env` since the phone was set up. |
| Upload appears to hang | The sim's uplink | The progress bar is real `XMLHttpRequest` upload progress, so **a moving bar means slow, not hung**. A motionless bar at 0% is the real symptom. |
| Upload returns **413** | Clip exceeds `max_upload_bytes` (2 GiB) | Almost certainly means 4K or a very long take — re-record at 1080p60 |
| Two clips landed in the wrong swing | Role-based assignment slots an upload into the *newest* swing missing that role; two swings simultaneously missing the same role can misattribute | Documented behaviour, not a crash. The escape hatch is the explicit `swing_id` query param, which overwrites a role slot by hand. |
| Nothing analysed after all three uploads | Bundle not actually complete — a role failed to land, or the same role was uploaded twice | Check the status panel for which roles the swing has. Re-upload the missing one. |
| Aligned video won't play on the phone | Codec fell back to `mp4v` | The results page says so explicitly when it happens; the clip needs VLC. Rare — the H.264 path normally wins. |
| Results look wrong on the down-the-line view | **Expected — this is the open question** | Phase detection was tuned on 461 *face-on* clips and has never been validated down-the-line; the lead wrist is the far arm and is occluded through the top. Don't debug it at the bay — record it and see §7. |

---

## 7. What to bring back

The bay trip is also the last unchecked box in M7. **Phase 0 — the field spike — has its method
locked and its thresholds pre-committed, and is waiting only on footage.**

- Read [M7_TWO_PHONE_SPIKE.md](M7_TWO_PHONE_SPIKE.md) before you go. Its three questions (does
  `segment_phases()` survive down-the-line, does OpenCV decode iPhone HEVC, what does
  `CAP_PROP_FPS` report for slo-mo) all get answered by footage from this session. The thresholds
  were written before any footage existed **on purpose** — don't renegotiate them after seeing
  the numbers.
- The worksheet at `spikes/2026-08-07-two-phone/log.md` is pre-printed for 11 swings in sets A–G.
  Fill it in as you go; reconstructing it afterwards from the clips does not work.
- One more unmeasured thing: **what iOS Safari does to a `.mov` on submit** — whether it
  re-encodes, strips metadata, or passes the file through. One real upload plus
  `spikes/2026-08-07-two-phone/probe.py inspect` answers it.
- **Funnel's real upload time**, if a helper's phone is used — the one blank row in §3.

M7's exit criteria, for reference: one bay session where every swing assembles from the right two
clips, the aligned side-by-side video's IMPACT banners land together in both panels, and shot data
attaches to the correct swing.
