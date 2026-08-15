"""Central configuration (the only place that reads the environment).

Modules receive config via this object rather than reading env vars themselves, which
keeps them pure and testable. Values can be overridden with env vars (prefix GOLF_) or
a local .env file.

Secrets are typed `SecretStr`, never `str`: `repr()`, `str()`, f-strings and `model_dump()`
all render `**********`, so a stray `print(settings)` or an exception traceback cannot spill
one. Reading a secret therefore costs an explicit `.get_secret_value()` — greppable, and
`tests/test_config.py` pins the exact set of files allowed to call it, so a fourth unwrap site
is a failing test rather than a quiet habit. OS keychains were considered for the *storage*
side and declined (ADR-019): platform lock-in in a project that is otherwise portable. The
values therefore still sit in plaintext in a gitignored `.env` at rest, which is the accepted
trade and the reason the masking above carries the weight it does.
"""

from __future__ import annotations

from pathlib import Path

from pydantic import Field, SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict

# Repo root = three parents up from this file (src/golf_coach/config.py -> repo root).
REPO_ROOT = Path(__file__).resolve().parents[2]


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="GOLF_", env_file=".env", extra="ignore")

    # Filesystem layout (data/ is gitignored).
    data_dir: Path = REPO_ROOT / "data"
    raw_dir: Path = REPO_ROOT / "data" / "raw"
    processed_dir: Path = REPO_ROOT / "data" / "processed"
    models_dir: Path = REPO_ROOT / "data" / "models"
    db_path: Path = REPO_ROOT / "data" / "golf_trainer.db"

    # Screen-capture shot ingestion (ADR-014). Drop photos of the launch monitor's
    # SHOT DATA screen in `shot_screens_dir`; parsed shots land in `shots_dir`.
    shot_screens_dir: Path = REPO_ROOT / "data" / "raw" / "shot_screens"
    shots_dir: Path = REPO_ROOT / "data" / "processed" / "shots"
    launch_monitor_profile: str = "hd_golf"
    ocr_engine: str = "paddle"
    # Below this, a parse is flagged `needs_review` rather than silently trusted.
    ocr_min_confidence: float = 0.6

    # Phone upload ingestion (trimmed M7 Phases 3+5) — swing bundles land here, one
    # directory per swing, grouped by role (face_on / down_the_line / shot_screen).
    sessions_dir: Path = REPO_ROOT / "data" / "processed" / "sessions"
    # Golfer registry (career mode, step 1) — one JSON file per golfer, keyed by `player_id`.
    # Sits beside `sessions_dir` rather than inside it: a golfer outlives any one session, and
    # that outliving is the entire point of tracking one.
    golfers_dir: Path = REPO_ROOT / "data" / "processed" / "golfers"
    # Follow-up conversations (M6, ADR-020) — one JSON transcript per conversation. Beside
    # sessions for the same reason as golfers: a conversation seeded from one swing can range
    # across sessions by its second turn, so filing it under that swing outgrows the directory.
    conversations_dir: Path = REPO_ROOT / "data" / "processed" / "conversations"
    # Guards the streamed upload from an unbounded body; ~2 GiB comfortably covers a
    # multi-minute 1080p60 clip.
    max_upload_bytes: int = 2 * 1024**3
    # Shared secret for phone uploads (ADR-016). Tailnet-only `tailscale serve` can run
    # without one; exposing the server any wider — Funnel, or a non-loopback bind —
    # requires it, and `scripts/run_server.py` refuses to start otherwise.
    # `SecretStr` implements `__len__`, so the plain truthiness checks that gate on this
    # (`scripts/run_server.py` refusing a non-loopback bind) keep their exact meaning: unset
    # and empty are both falsy. That is load-bearing — it is a security refusal, not a hint.
    upload_token: SecretStr | None = Field(default=None)
    # The background analysis worker. On by default: an upload that lands and does nothing is
    # the state M7 Phase 5 exists to fix. Turn it off to run the server as pure ingestion and
    # drive `scripts/analyze_bundle.py` by hand — which is also what the API tests do, so a
    # test never has to own a running event-loop task it didn't ask for.
    analysis_enabled: bool = True

    # Service ports (see docs/ARCHITECTURE.md deployment view). api_port is the loopback
    # port Tailscale Serve proxies to, never a port a phone connects to directly.
    # 3000 rather than 8080: Windows reserves 8069-8168 (and 7969-8068, 8169-8268), so
    # binding 8080/8000/8443 fails here with WinError 10013. Check with
    # `netsh interface ipv4 show excludedportrange protocol=tcp` before changing.
    api_port: int = 3000
    mcp_port: int = 8081

    # LLM coaching (M6). Read from env; never hardcode.
    anthropic_api_key: SecretStr | None = Field(default=None)
    # Latest, most capable Claude model for coaching (see /claude-api skill before changing).
    coaching_model: str = "claude-opus-5"
    # On by default, because the API key is the real gate: with no key the pipeline skips the
    # call and says so in `notes`, so nobody is billed for installing the repo. Turn it off to
    # run the pipeline without coaching even when a key is configured (`--no-coaching`).
    coaching_enabled: bool = True


settings = Settings()
