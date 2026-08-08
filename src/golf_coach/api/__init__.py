"""API module — FastAPI upload server (the imperative shell for phone ingestion).

Currently fills only the upload -> swing bundle store seam (`api/app.py`): a phone
browser posts a file tagged with a role, and the file lands on disk grouped into
the right swing. It does not yet wire capture -> pose/detection -> analysis ->
feedback; that orchestration is a later phase. Requires the `api` extra.
"""
