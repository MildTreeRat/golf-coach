# ADR-019: Secret handling — masked in memory, plaintext at rest, and why the keychain was declined

## Status
Accepted

## Date
2026-08-14

## Context

This repo holds two secrets: `GOLF_UPLOAD_TOKEN`, the shared bearer secret gating `/api/`
([ADR-016](016-local-first-host-and-phone-upload-topology.md)), and `GOLF_ANTHROPIC_API_KEY`, which
pays for the M6 coaching call. Until now both were `str` fields in `config.py` loaded from a
gitignored `.env`, and the working assumption was that `.gitignore` was the protection.

It is not. `.gitignore` answers exactly one question — *does this reach the public repo?* — and
that repo **is** public, so the answer matters. But it says nothing about the two questions that
actually bound the risk:

| Axis | Question | What `.env` + `.gitignore` does |
|---|---|---|
| **Disclosure** | Can the value reach stdout, a log, a traceback, a `model_dump()`? | nothing |
| **At rest** | Who can read the bytes on disk? | nothing — it *is* the plaintext |

Three measurements taken on 2026-08-14, which is what turned this from a tidy-up into a decision:

- **`.env`'s ACL granted `BUILTIN\Users` → `Modify`.** Every local account could read both secrets.
- **`.gitignore` protected exactly one filename.** The bare `.env` rule did not match `.env.bak`,
  `.env.local`, or `.env.save` — so `cp .env .env.bak` before an edit would have committed a live
  token to a public repo.
- **`scripts/run_server.py` printed the upload token in full** on every start, into scrollback that
  outlives the session and into any screenshot or screen share of a bay session.

The prompting event was an API key pasted into an agent session, which put it in a plaintext
transcript outside the repo entirely. That is not a failure mode any code change prevents — but it
is a clean demonstration that the interesting leaks are the ones that route *around* `.gitignore`.

## Options Considered

### Option A: keep `str` + `.env` + `.gitignore`, fix only the ignore rules
- **Pros**: zero code change; nothing new to learn.
- **Cons**: leaves the disclosure axis entirely unaddressed. A `print(settings)` added while
  debugging, or any exception whose traceback carries the settings object, spills both secrets. The
  masking cost is one annotation; declining it buys nothing.

### Option B: `SecretStr` for the disclosure axis
- **Pros**: `repr()`, `str()`, f-strings and `model_dump()` all render `**********` (verified, not
  assumed). Zero new dependencies — `pydantic` is already a base dep. Platform-independent. Turns
  reading a secret into an explicit `.get_secret_value()`, which is greppable and therefore
  *pinnable*.
- **Cons**: does nothing at rest. Not a boundary a determined caller cannot cross — it raises the
  cost of an accident, not of an attack.

### Option C: OS keychain (`keyring` → Windows Credential Manager / DPAPI)
- **Pros**: removes plaintext-at-rest. Survives repo deletion, cloud sync, and other local accounts.
  `config.py` is already the only module that reads the environment, so the resolution chain would
  have been a one-file change.
- **Cons**: **platform lock-in.** `keyring` is cross-platform in name, but its Windows backend *is*
  Windows Credential Manager, so on this machine the secret store becomes an OS component in a
  project that is otherwise portable and stdlib-first. It also adds a dependency to a repo whose
  base install is deliberately thin, and it does not defend against the realistic local threat —
  malware running as the same user must be able to decrypt whatever the app can decrypt.

### Option D: encrypted file with a passphrase (`age`, `sops`, Fernet)
- **Pros**: portable, no OS coupling, genuinely removes plaintext-at-rest.
- **Cons**: needs a passphrase at startup. `AnalysisWorker` runs unattended behind the upload
  server, so this trades an unattended service for a manual one — the wrong trade for a home lab
  whose whole point is that a swing uploaded from the bay analyses itself.

## Decision

**Option B, plus the file-level hardening from Option A. Option C is declined for platform
lock-in; Option D for the unattended-startup cost.**

Concretely:

- `upload_token` and `anthropic_api_key` are `SecretStr | None` in `config.py`.
- Unwrapping is allowed in exactly three files, each a boundary where an outside caller needs a
  plain `str`: `api/pipeline.py` (the Anthropic SDK), `api/app.py` (`secrets.compare_digest`), and
  `scripts/run_server.py` (the operator's setup link). `tests/test_config.py` pins that set.
- `.gitignore` matches `.env*` with `!.env.example`, plus `*.key`/`*.pem`/`*.pfx`/`*.p12`.
- `scripts/run_server.py` prints a four-character token prefix; `--show-token` prints the link in
  full.
- Secrets stay in a gitignored `.env` at rest. **This is the accepted risk, not an oversight.**

## Consequences

- **Masking now carries weight it did not before.** Having declined the at-rest fix, the disclosure
  fix is most of the remaining defence — which is why it is pinned by tests rather than left to
  care. `tests/test_config.py` fails if either annotation reverts to `str`, and fails if a fourth
  file learns to unwrap.
- **A new unwrap site is a deliberate act.** It costs a line in `SANCTIONED_UNWRAP_SITES` and a
  note here. That friction is the feature.
- **Anyone with the machine has the secrets.** Local plaintext is the trade. The mitigations are
  filesystem permissions (`icacls` / `chmod 600`, applied once, not enforced by code) and the fact
  that this is a single-user home lab.
- **Rotation stays the only revocation.** Unchanged from ADR-016 for the upload token, and the same
  is now true of the Anthropic key: there is no per-device identity to revoke.
- **`echo "..." >> .env` is out.** On Windows, PSReadLine logs every command to a plaintext history
  file that survives reboots, so the documented setup step became "edit the file". `README.md` and
  `ROADMAP.md` both say so, and `.env.example` repeats it where it will actually be read.
- **`SecretStr` truthiness is load-bearing.** It implements `__len__`, so `not settings.upload_token`
  in `scripts/run_server.py` keeps its exact meaning — unset and empty both falsy. That refusal is a
  security check, so `tests/test_config.py` pins the falsiness rather than trusting it to read as
  obvious.
- **What this does not fix**: `api/app.py` still waves every request through when no token is
  configured, and the only guard against a wide-open bind lives in `scripts/run_server.py` — so any
  other entry point (`uvicorn ... --host 0.0.0.0`, a container, a systemd unit) bypasses it. That is
  a real hole, deliberately out of scope here because closing it changes API behaviour and cuts
  against ADR-016's premise that tailnet membership *is* the access control. It needs its own ADR.

## Addendum (2026-08-15): a fourth unwrap site, and the pin doing its job

`SANCTIONED_UNWRAP_SITES` gained **`scripts/ask_swing.py`** ([ADR-020](020-conversational-followups.md)).
Recorded here because the Consequences above say a new site costs a line in the set and a note in
this file, and a rule whose first exercise goes unrecorded is a rule that stops being followed.

**Why it is the right shape.** It is the same boundary as the three already listed: the Anthropic
SDK takes a plain `str` and cannot be handed a wrapper. The CLI is the follow-up conversation's
entry point exactly as `api/pipeline.py` is the one-shot coaching call's, and it unwraps at the
call site rather than passing a `SecretStr` down — `feedback/conversation.py` takes `api_key: str
| None` for the same reason `generate_coaching` does, which keeps the module that talks to the
model ignorant of how the key is held.

**The web half needed nothing.** `api/app.py`'s follow-up route was already on the list for
`compare_digest`, so the conversation added no site there.

**Worth noting how this surfaced**: the pin failed the first time the full suite ran after the
work, naming the added file and pointing at this document. That is the friction the original
decision described as the feature, working on its first real test.

## Addendum (2026-10-22): `feedback/coach.py`, the paragraph's entry over `golf-mcp`

`SANCTIONED_UNWRAP_SITES` gained **`src/golf_coach/feedback/coach.py`** (M29 P17,
[the lab-port plan](../plans/m29-lab-port.md)). Its new entry,
`python -m golf_coach.feedback.coach --session S --swing W`, reads the swing through the Rust
`golf-mcp` server, asks Claude for the one-shot coaching paragraph, and prints it as one JSON
object. `golf-lab analyze` spawns it from M29 P26, because Python stays the only caller of the LLM
([ADR-035](035-rust-everywhere-python-where-required.md) clause 1). The entry's `main()` unwraps the
key at that call.

**Why it is the right shape.** It is the same boundary as `scripts/ask_swing.py` in the addendum
above. The Anthropic SDK takes a plain `str` and cannot be handed a wrapper, and an entry point
unwraps at the call site rather than passing a `SecretStr` down. `coach_through_golf_mcp` takes
`api_key: str | None`, as `generate_coaching` and `feedback/conversation.py` do, so the code that
talks to the model still does not know how the key is held. The SDK now has three unwrap sites:
the paragraph in process (`api/pipeline.py`), the paragraph over `golf-mcp` (this one), and the
conversation (`ask_swing.py`).

**It replaces a site rather than widening the surface for good.** `api/pipeline.py`'s site goes when
M40 deletes that file, and from then on this entry is the paragraph's only boundary. The set is one
larger only for M29 to M40, while both routes to the paragraph exist.

**Two alternatives were rejected.**

- *Reading the key as `getattr(key, "get_secret_value")()`* would get past the `ast` pin, and
  getting past it is exactly what the pin exists to stop.
- *Having `golf-lab` pass the key in the child's environment* would bring the secret into Rust.
  Rust reads no `.env`, and this decision keeps the key in one process that already holds it.

**How it surfaced**: the same way as the 2026-08-15 addendum. The full suite failed on the pin,
naming the added file, and the phase stopped to ask rather than add the line itself.

## Addendum (2026-11-09): `evals/runner.py`, the eval runner's entry

`SANCTIONED_UNWRAP_SITES` gained **`src/golf_coach/evals/runner.py`** (M43 P7,
[the plan](../plans/m43-llm-evals.md)), as
[ADR-036](036-llm-evaluation-and-observability.md) §5 said it would. Its entry,
`python -m golf_coach.evals.runner --flow F --model M`, runs an eval's cases through the real
routes, and builds its own `anthropic.Anthropic(max_retries=0)` so that every retry is the
runner's and shows in its rows. That client resolves `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` or
an `ant auth` profile, as the SDK does. Only when the SDK finds none of them does `_build_client`
unwrap `settings.anthropic_api_key` (`GOLF_ANTHROPIC_API_KEY`), at the entry point.

**Why it is the right shape.** It is the same boundary as the entries above. The Anthropic SDK
takes a plain `str` and cannot be handed a wrapper, and an entry point unwraps at the call. The
runner hands the client it built to each route through `client=`, so `look_up_set` and
`coach_through_golf_mcp` never see the key in any form, and their own key gates never run. The
SDK now has four unwrap sites: the paragraph in process (`api/pipeline.py`), the paragraph over
`golf-mcp` (`feedback/coach.py`), the conversation (`scripts/ask_swing.py`) and the eval runner
(this one).

**It widens the surface, where the 2026-10-22 entry replaced a site.** It is kept narrow three
ways. A run is a command the user approves, never a test (ADR-036 §5 and §6). The tests hand the
runner a fake client and a fake SDK, so `pytest` never unwraps the key there. And the key is read
only when the SDK resolved nothing of its own.

**Two alternatives were rejected.**

- *Resolving only `ANTHROPIC_API_KEY` or an `ant auth` profile* needs no unwrap. But the key the app
  already keeps in `.env` would not run an eval, and M43 P13's precondition would lose
  `GOLF_ANTHROPIC_API_KEY`. ADR-036 §5 chose the fallback, and the user approved that clause on
  2026-11-09.
- *Copying the settings key into `ANTHROPIC_API_KEY` in the process environment* would let the SDK
  find it with no unwrap the pin can see. That is the `getattr` evasion above by another name, and
  every child process, the `golf-mcp` server the coach flow starts included, would inherit the key.

**How it surfaced**: planned rather than caught. ADR-036 §5 named the site before the code existed,
and the full suite still failed on the pin when the runner landed, naming the added file. The line
and this addendum went in with the runner.
