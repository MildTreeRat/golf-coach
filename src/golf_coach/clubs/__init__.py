"""Club specifications — what the manufacturer says a club is. [M12]

A new subsystem rather than a corner of an existing one, because it answers a question none of the
others do. `analysis/` measures a swing, `storage/` keeps what happened, `feedback/` says something
about it; this package looks up a *published property of a manufactured object* — a thing nothing
in this bay can measure and nothing needs to (ADR-026 §Context).

Two modules, and they are a cache and its miss path:

- `catalogue.py` — the committed catalogue, `club_catalogue.json`, read by dictionary lookup. Ships
  inside the package as data with provenance per row, which is ADR-022's pattern rather than a new
  one.
- `lookup.py` — the LLM half, called only when the catalogue misses. [P4]

Imports `contracts` only, and nothing here imports `analysis`, `storage` or `api` (ADR-008).
`api/app.py` is what composes this with `BagStore`.
"""
