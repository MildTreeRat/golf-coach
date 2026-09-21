# ADR-001: Primary Language — Python

## Status
**Superseded by [ADR-030](030-app-platform-rust-core-python-sidecar.md)** (2026-09-21) on the
primary-language question. Accepted 2026-03-16 and correct for what it judged: a desk pipeline,
where development speed dominated and nothing had to be installed by anyone. ADR-030 chooses Rust
for a *shipped app* that records two camera streams while analysing in the background — a
different subject, not a reversal.

**What survives is Option A's actual finding — the ML and CV ecosystem is Python's** — and that is
why ADR-030 keeps pose in MediaPipe-Python rather than porting it. Python remains the lab: fitting
under the `research` extra ([ADR-022](022-learned-artifacts-as-committed-data.md)), the corpus
tools, the conformance oracle, LLM coaching and OCR. What does not survive is "all backend".

The Decision's other clause — *"JavaScript/React for the web UI only"* — never happened. What got
built is five hand-written HTML pages with no framework and no build step, which
[REFACTOR_LEDGER.md](../REFACTOR_LEDGER.md) has twice declined to revisit. ADR-030 replaces that
clause with Flutter rather than fulfilling it.

## Date
2026-03-16

## Context
Need to choose a primary language for the project. The system involves computer vision, machine learning model training and inference, data processing, and API development.

## Options Considered

### Option A: Python
- **Pros**: Dominant ML/CV ecosystem (PyTorch, MediaPipe, OpenCV, Ultralytics all Python-first). FastAPI for backend. Huge community and learning resources. Rapid prototyping.
- **Cons**: Slower runtime than compiled languages. GIL limits true parallelism (mitigated by multiprocessing and async).

### Option B: C++ / Rust
- **Pros**: Maximum performance for real-time processing. Direct hardware access.
- **Cons**: Much slower development cycle. ML ecosystem is secondary. Overkill for a home lab project.

### Option C: JavaScript/TypeScript (full stack)
- **Pros**: Single language for backend + frontend. Good for UI.
- **Cons**: ML ecosystem is immature. No good pose estimation or object detection libraries.

## Decision
**Python** for all backend, ML, and data processing. JavaScript/React for the web UI only. This gives us the best ML ecosystem with minimal friction.

## Consequences
- All ML work, data pipelines, API, and MCP server are Python.
- Frontend is a separate React app communicating via REST API.
- Need to manage two language environments (Python venv + Node for UI).
