# Frontend (React) — placeholder

The web UI lives here (Milestone 5). Kept as a separate top-level directory with its own
toolchain (npm/node), intentionally **not** under `src/` — it talks to the Python backend
over HTTP (FastAPI, see `src/golf_coach/api/`), which keeps the two decoupled.

Scaffolding (Vite/CRA, components, etc.) is added in M5.
