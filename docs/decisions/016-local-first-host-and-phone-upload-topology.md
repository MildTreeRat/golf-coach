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
