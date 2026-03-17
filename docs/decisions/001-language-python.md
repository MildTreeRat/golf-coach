# ADR-001: Primary Language — Python

## Status
Accepted

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
