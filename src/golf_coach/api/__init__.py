"""API module — the phone-upload server and the imperative shell around the analysis core.

Three pieces (M7 Phase 5):

- `app.py`     FastAPI routes: streamed upload, session/swing status, the results feed, video
- `pipeline.py` swing bundle -> `analysis.json` + `aligned.mp4`; the orchestration ADR-008
                puts in `api/`, shared with `scripts/analyze_bundle.py`
- `worker.py`   an in-process asyncio queue that runs the pipeline when a bundle completes

`app.py` needs the `api` extra; `pipeline.py` deliberately does not, so the CLI can use it on a
`vision`-only install (see `tests/api/test_pipeline_imports.py`).
"""
