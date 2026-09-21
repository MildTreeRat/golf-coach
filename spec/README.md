# `spec/` — what a second implementation is checked against

> **Tier: AS-BUILT.** Everything here is generated from the code beside it and re-checked by
> `tests/test_conformance.py` on every run.

**Read [`docs/CONFORMANCE.md`](../docs/CONFORMANCE.md) first.** It is the specification; this is
the data, and none of it is hand-written.

```
spec/
  schemas/    JSON Schema per shape that crosses the seam, exported from src/golf_coach/contracts/
  vectors/
    synthetic/   deterministic fixtures — small, readable, cover the code paths
    corpus/      the 15 real swings on disk, gzipped — cover the numerics
```

```bash
python scripts/conformance.py check    # every vector against this build
python scripts/conformance.py list     # what is here, and where it came from
```

Do not edit a file in this directory. `python scripts/conformance.py regenerate` rewrites all of
it, and an edit that survives a regeneration is an expectation nothing produced.
