"""Central configuration (the only place that reads the environment).

Modules receive config via this object rather than reading env vars themselves, which
keeps them pure and testable. Values can be overridden with env vars (prefix GOLF_) or
a local .env file.
"""

from __future__ import annotations

from pathlib import Path

from pydantic import Field
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

    # Service ports (see docs/ARCHITECTURE.md deployment view).
    api_port: int = 8080
    mcp_port: int = 8081

    # LLM coaching (M6). Read from env; never hardcode.
    anthropic_api_key: str | None = Field(default=None)
    # Latest, most capable Claude model for coaching (see /claude-api skill before changing).
    coaching_model: str = "claude-opus-4-8"


settings = Settings()
