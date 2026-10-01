# ADR-016: Local-First Host & Phone Upload Topology

## Status
Accepted

## Date
2026-08-06

## Context
M7 Phase 5 built the upload server: a static page with a role picker, and `POST /api/uploads`
streaming a clip to disk. It binds `127.0.0.1`, so the only thing that can reach it is the
desktop's own browser. That makes the feature useless for its actual purpose — the swing is
recorded on phones, at an indoor sim, on a network that is not the home LAN.

So the question is how a phone reaches the desktop. Two facts constrain it:

**The endpoint is unauthenticated and writes to disk.** `POST /api/uploads` accepts up to 2 GiB
(`config.py`) and streams it straight into `data/processed/sessions/`. Any exposure mechanism that
does not come with an access-control story turns this into an open write-to-my-disk endpoint.

**One of the phones may not be mine.** M7 is a two-person capture — one person films face-on, the
other down-the-line. The second phone often belongs to whoever is at the bay. This is the fact that
kills the obvious answer: a private mesh VPN is excellent for devices I own and unusable for a
device I am borrowing for ninety seconds.

Clip sizes set the transfer budget. Measured: `data/raw/aaron-swing-2.mov` is 2.6 MB; a 10 s
1080p60 HEVC iPhone clip lands ~30–50 MB. The 2 GiB config ceiling is a guard against a runaway
body, not a description of normal traffic. But M7 explicitly wants headroom to record 4K when the
phase detector needs it, and upload time over the bay's uplink is already the dominant latency in
the whole system — not compute.

## Options Considered

### Option A: Bind uvicorn to `0.0.0.0` on the home LAN
- **Pros**: Zero new software. Works today.
- **Cons**: Only works on the home WiFi, which is the one network the phones are *not* on. Also
  publishes an unauthenticated upload endpoint to every device on the LAN.

### Option B: Router port-forward + dynamic DNS
- **Pros**: Free, no third party in the data path, full bandwidth.
- **Cons**: Puts an unauthenticated endpoint on the public internet behind nothing but a port
  number. Needs a DDNS account, router config, and a TLS certificate story of its own. The most
  operational work and the worst security posture of any option here.

### Option C: Cloudflare Tunnel
- **Pros**: Free. Public HTTPS URL with no app on the phone — which solves the borrowed-phone
  problem directly. No inbound ports opened.
- **Cons**: **Free and Pro plans enforce a hard 100 MB request-body limit**, returning HTTP 413 at
  the edge before the request ever reaches the origin. It is documented infrastructure behaviour,
  not a setting. A 1080p60 clip usually fits; a 4K clip or a long take fails, and the failure
  arrives as an edge error rather than anything the app can see. Working around it means building
  client-side chunked upload and server-side reassembly — real complexity bought purely to satisfy
  a vendor limit. Also a second tool, a second account, and a second certificate to maintain.

### Option D: Tailscale, bound to the tailnet IP (the original Phase 6 plan)
- **Pros**: Free. Encrypted, direct WireGuard peer-to-peer with no relay and no bandwidth cap.
  Tailnet membership is a real access-control boundary, which is what lets the endpoint stay
  unauthenticated.
- **Cons**: Plain HTTP, so no secure context — which forecloses `getUserMedia` if the page ever
  captures video directly instead of picking a file. And it does nothing for the borrowed phone:
  a helper cannot join my tailnet.

### Option E: Tailscale Serve for my devices, Funnel for guests (chosen)
- **Pros**: One install, one certificate, one loopback bind, two reach levels. Serve publishes to
  the tailnet; Funnel publishes the same URL to the public internet on demand. Both terminate real
  Let's Encrypt TLS and proxy to `127.0.0.1`, so the bind never widens and a secure context comes
  free. Funnel documents no request-body limit. Free on all plans, including Personal.
- **Cons**: Funnel is relayed through Tailscale with undisclosed, non-configurable bandwidth
  limits — slower than Serve's direct path, and not a documented guarantee. Requires a `funnel`
  node attribute in the tailnet policy file, and only listens on 443 / 8443 / 10000. Crucially, it
  makes the upload endpoint publicly reachable, so it cannot be run without authentication.

## Decision
**Option E.** Tailscale in front of an unchanged loopback bind, in two modes:

| Mode | Command | Reachable by | Path |
|---|---|---|---|
| Serve (default) | `tailscale serve --bg 3000` | my own devices | direct WireGuard, unmetered |
| Funnel (on demand) | `tailscale funnel --bg 3000` | anyone with the link | relayed, rate-limited |

The deciding factor is that the two problems have different shapes and Tailscale solves both
without a second tool. My phones are on the tailnet and should get the fast direct path. A guest's
phone needs a plain URL that works with no app, accepts the slower relayed path, and is turned off
again afterwards. Cloudflare would have handled the guest case at the cost of a 100 MB wall on
*every* upload including my own — paying the worse case's price in the common case.

### The bind never widens
uvicorn stays on `127.0.0.1` in all modes. Tailscale terminates TLS and proxies inward, so
"reachable from a phone" is never implemented by changing the bind address. This is strictly safer
than what this ADR's own Option D proposed, and it makes the dangerous configuration
unreachable-by-construction rather than warned-against-in-a-docstring.

`scripts/run_server.py` enforces it: a non-loopback `--host` with no token configured is refused
with a non-zero exit rather than started.

### Tailnet membership is not enough once Funnel exists
Phase 5's premise was that tailnet membership *is* the access control. That holds for Serve and
collapses for Funnel. So `/api/` routes are gated on a shared secret, `GOLF_UPLOAD_TOKEN`:

- Enforced only when set. Unset means tailnet-only Serve, where the original premise still holds.
- Accepted as the `X-Upload-Token` header or a `?t=` query param. The query param exists so a phone
  can be set up by opening one link; the page stores the token in `localStorage` beside the role it
  already remembers, strips it from the URL via `history.replaceState`, and sends the header
  thereafter. Same one-time-setup-per-phone shape as the role picker.
- Implemented as a **route dependency, not middleware**, so it resolves before the handler touches
  `request.stream()`. A rejected upload never writes a byte to `.incoming/`.
- Compared with `secrets.compare_digest`.
- The static page stays ungated — it holds no data, and gating it would break the `?t=` bootstrap
  that teaches the phone the token.

This is a shared bearer secret, not per-device identity. Revocation means rotating the token and
re-opening the setup link on each phone. For two-to-three phones and a home lab that is the right
weight; anything finer would be building an account system for a garage.

## Consequences
- **Phase 6's exit criterion is met**: a phone on cellular, WiFi off, uploads a swing.
- **The default port moved from 8080 to 3000.** Not cosmetic — Windows reserves 8069–8168 on this
  machine, so 8080 could never bind (`WinError 10013`); 8000 and 8443 are inside adjacent reserved
  ranges. Verify with `netsh interface ipv4 show excludedportrange protocol=tcp` before changing.
  The port is now internal anyway: it is what Tailscale proxies to, never what a phone connects to.
- **A secure context is now available**, so a future page can use `getUserMedia` to capture in-app
  instead of round-tripping through the camera roll. Not built; no longer blocked.
- **Funnel is a deliberate, temporary act.** It is off by default, turned on for a session with a
  guest phone, and turned off after. Leaving it on is the single riskiest state this design
  permits, which is why the token is mandatory rather than advisory.
- **The relay is a bandwidth risk for guest uploads only.** If Funnel proves too slow at the bay,
  the fallback is asking the helper to AirDrop the clip and uploading it from a tailnet device —
  degraded, not broken.
- **Still no cloud, no phone app, no open router port**, consistent with ADR-014's offline-first
  stance. The clips never leave hardware I own; Funnel relays bytes but stores nothing.
- **This does not supersede ADR-011.** Camera topology is unaffected — this ADR is only about how
  files arrive.

## Addendum (2026-08-14): how the token is *held*, now decided separately in ADR-019

This ADR settled what the upload token **is** — a shared bearer secret in a URL, compared with
`secrets.compare_digest`, revoked only by rotation. All of that stands unchanged.

What it never stated is how the value is stored and rendered, and the gap showed:
`scripts/run_server.py` printed it in full on every start, into scrollback that outlives the
session and into any screenshot of a bay session. [ADR-019](019-secret-handling.md) now owns that
question for both of this repo's secrets. Two consequences land here:

- `settings.upload_token` is a `SecretStr`, so it renders as `**********` everywhere except the one
  sanctioned unwrap in `api/app.py`. The truthiness checks this ADR's design depends on — chiefly
  `scripts/run_server.py` refusing a non-loopback bind on `not settings.upload_token` — are
  unaffected, because `SecretStr` implements `__len__`. `tests/test_config.py` pins that.
- The startup line prints a four-character prefix; `--show-token` prints the setup link in full.

The `?t=` query parameter, the `localStorage` handoff, and rotation-as-revocation are all as
described above. This narrows one operational edge; it does not reopen the design.

## Addendum (2026-09-21, M18): a phone app, after all — and everything else here holds

[ADR-030](030-app-platform-rust-core-python-sidecar.md) builds an installed app, and a downloadable
phone app with it. That contradicts exactly **one clause of one bullet** above — *"Still no cloud,
no phone app, no open router port"* — and nothing else in this ADR. The phone app supersedes "no
phone app". The other two stand, and are reinforced rather than merely survived:

- **Still no cloud.** ADR-030 §7 closes remote analysis outright rather than deferring it, on this
  ADR's and [ADR-014](014-screen-capture-shot-ingestion.md)'s offline-first grounds. The clips
  still never leave hardware the golfer owns.
- **Still no open router port.** The phone talks to a laptop on the same network; Funnel remains
  what it is, off by default and turned on for a guest phone.

**The door this ADR left open is the one being walked through, at a different angle.** The
Consequences above note that a secure context makes `getUserMedia` possible — *"Not built; no
longer blocked."* ADR-030 does not take that route: it puts a native app on the phone instead of a
capture page, because the phone needs to run the strike detector to avoid streaming video. The
observation was right that in-app capture had stopped being blocked; the mechanism is native rather
than browser.

**What the phone sends changes, and the upload design mostly does not.** Today a golfer records with
the stock camera app and uploads from the camera roll. Under ADR-030 the phone records continuously
and sends a ~10-second clip when its own detector hears a strike — encoded by the phone's hardware
encoder at roughly 10-20 MB, never raw frames and never a continuous stream. The two views still
need no shared clock in flight, because [ADR-025](025-acoustic-synchronization.md) recovers the
offset acoustically afterwards.

**Upload does not go away.** ADR-030 §6 makes file upload a first-class capture source that must
keep working after cameras exist — it is how the system stays testable before any hardware is
bought, and the only source that can replay the stored corpus through the new stack. This addendum
narrows one clause; it does not retire the topology.

## Addendum (2026-09-30, M31): the phone is the host and listens on nothing, and this topology is the lab's

[ADR-034](034-shot-first-phone-first.md) makes a standalone iPhone the host. It photographs the
screen, reads it, stores the shot and grades it, with no laptop at the bay. This ADR answered *how a
phone reaches the desktop*, and on the product's path the phone no longer needs to. Each point
routes to an ADR-034 clause rather than restating it. **Nothing here is built**; M38 builds the
phone's side.

**The phone is the host, and nothing on it listens on a port**
([clause 6](034-shot-first-phone-first.md#6-the-phone-is-the-host)). Storage is on the device, and a
session completes with no network at all. That is the local-first posture Option E chose, in its
strongest form: there is no bind to widen, because nothing binds, and no endpoint for a token to
gate.

**Export is how data leaves the phone** (M38 P4), and the lab imports it with
`scripts/import_phone_export.py`. How the file travels from the phone to the laptop is M38 P4's to
settle. Nothing here assumes it is this ADR's upload route.

**This topology stays the lab's path until M40 says otherwise.** Serve for the golfer's own devices,
Funnel on demand for a guest's, the loopback bind, the refused non-loopback start and
`GOLF_UPLOAD_TOKEN` are all unchanged. They are still how a clip or a photo reaches the laptop's
`api/` today, and
[ADR-030's addendum of 2026-09-29](030-app-platform-rust-core-python-sidecar.md#addendum-2026-09-29--the-machine-is-the-phone-what-adr-034-superseded-here-and-what-it-left)
keeps file upload first-class on the laptop, because it is the only source that replays the corpus.
What the laptop client needs from this ADR is
[M40](../plans/m31-m40-shot-first-pivot.md#m40--the-laptop-client-resumes)'s to decide. Until then
the topology lasts as long as `api/` does, and M29 retires `api/` once M40 has landed.

**[ADR-019](019-secret-handling.md)'s API key never reaches the phone.** It pays for an LLM call,
and the phone makes none ([clause 10](034-shot-first-phone-first.md#10-no-llm-coaching-on-the-phone)).
The upload token still gates the laptop's `/api/`, and the phone serves nothing a token could gate.
The first addendum's rules for holding the token are unchanged.

**The previous addendum's picture of the phone is superseded; this ADR's body is not.**

- *"The phone talks to a laptop on the same network"*, and *"What the phone sends changes"*, where
  the phone records continuously and sends a clip at each strike, described ADR-030 §5's phone app.
  ADR-034 supersedes §5, and M28, which was to build phone-over-Wi-Fi capture, is superseded by M39,
  where the phone keeps its own clip.
- *"Upload does not go away"* still holds, on the laptop (above).
- The body's one superseded clause, "no phone app", is still the only one. The rest of the body
  describes the lab, which still works that way.

**Reinforced: no cloud, and no open router port.** The previous addendum reinforced both against a
phone that talked to a laptop. A phone that talks to nothing holds both without effort: no data
leaves hardware the golfer owns except by an export they make, and ADR-034 reinforces ADR-030 §7's
closed cloud in turn, because the phone needs no network.

**Not changed**:

- the Decision, *The bind never widens* and *Tailnet membership is not enough once Funnel exists*,
  all on the laptop;
- the first addendum: how the token is held is ADR-019's question;
- Funnel as a deliberate, temporary act, off by default.

## Addendum (2026-09-30, M31.5): M40, not M29, decides `api/`, and the phone export is a verb of the Rust lab

[ADR-035](035-rust-everywhere-python-where-required.md) ports to Rust everything that has no required
library, and the lab with it, in a re-scoped M29
([clause 5](035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)). Two
sentences of the previous addendum became false: one names the wrong milestone, and the other names
a Python script that will not be written. Each point routes to an ADR-035 clause rather than
restating it. **Nothing here is built.**

**"M29 retires `api/` once M40 has landed" now means that M40 decides `api/` and deletes it.** ADR-035
splits retirement into two moments (clause 5, and Q17 of
[M31.5 P2](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30)):

- **M29 replaces the lab's entry points** with a Rust lab CLI. It adds its Rust routes beside the
  frozen Python ones, and it neither rewires nor retires the FastAPI server. M29 is also no longer
  blocked on M40, so it now lands first, and the old sentence's order does not hold either.
- **M40 decides `api/`**: it ports it to `axum` or drops it
  ([clause 2](035-rust-everywhere-python-where-required.md#2-everything-else-ports-including-the-three-things-considered-and-not-kept)).
  Either way, M40 deletes the Python server and everything it imports.

So this topology still lasts as long as the FastAPI server does, and that is until M40. If M40 ports
`api/` to `axum`, the port decides whether it keeps this ADR's rules: the loopback bind, the refused
non-loopback start, and the token as a route dependency that resolves before a byte is written. It
records that decision here, by addendum. If M40 drops `api/`, the rules go with it.

**The phone export is imported by the Rust lab CLI, not by `scripts/import_phone_export.py`** (Q8,
clause 5). The previous addendum named that script, and it was planned as new Python. Under ADR-035
there is no new Python, so the import becomes a verb of M29's lab CLI. Two things follow:

- M38 P4 waits on M29.
- M38 P4's exit compares what was imported against the Rust corpus reader, not Python's
  `read_corpus`.

How the file travels from the phone to the laptop is still M38 P4's question, and it still assumes
nothing about this ADR's upload route.

**After M29, the upload route is the one path by which Python still writes `data/`.**
`POST /api/uploads` hands a clip to `api/worker.py`, which runs `api/pipeline.py`'s frozen analysis
over it (read 2026-09-30). From M29 the Rust lab writes `data/`. A swing that arrives by this route is
written by frozen Python instead, at `ANALYSIS_VERSION` 16 and without the keys only Rust writes
([clauses 3](035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust) and
[4](035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)). M29 decides whether the
route stays open between M29 and M40, and ADR-035 defers that question to it by name. Until M29, the
route is the lab's path exactly as the previous addendum describes it.

**Not changed**:

- the Decision, *The bind never widens* and *Tailnet membership is not enough once Funnel exists*,
  for as long as the server they guard exists;
- the first addendum: how the token is held is still ADR-019's question;
- the previous addendum's phone, which is the host, listens on nothing and exports. ADR-035 changes
  which language imports the export, and not what the phone does;
- no cloud, and no open router port.
