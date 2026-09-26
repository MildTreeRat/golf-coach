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
    audio/       one per stored clip — the ball-strike detector's oracle          [M20]
    stages/      one per engine vector — the intermediates, so a port has a gate
                 per stage instead of one gate at the end                         [M22 P1]
    format/      CPython's own rounding, float formatting and dict ordering —
                 the three edges a Rust port diverges on                          [M22 P3]
```

The first two are the **engine** families: `conformance.py` is their implementation and `check`
runs them. The other three name a Rust crate, so `check` reports them and defers, and all three are
live under `cargo test` — `crates/trigger` runs `audio/`, `crates/analysis` runs `format/` and
**all seven of `stages/`'s stages**, and `crates/core` runs the engine families themselves, all 21
vectors end to end. That last one closed M22 (P8b): the port these were written for conforms, so
what is here now is the gate a *second* port is held to. `docs/CONFORMANCE.md` §2's table says
which stage gates what.

A green stage is worth less than it looks, which M22 P4 measured and every phase after it
re-measured: the vectors gate the path this corpus takes, and a guard that is inert on all 21
swings — or a refusal branch none of them reaches — is the crate's own unit tests to cover. That
gap is not small and it is not guessable from the file count; see that same section, and ADR-032's
closing addendum for the whole list of it.

`format/` is the one family whose subject is not this engine, so it is also the only one that does
not age on `ANALYSIS_VERSION` — a rounding rule belongs to the language, and what it carries is
`python_version`.

```bash
python scripts/conformance.py check    # every engine vector against this build
python scripts/conformance.py list     # what is here, and where it came from
cargo test                             # the three families check defers, and the engine
                                       #   vectors end to end against the port
cargo run --bin golf-core -- run < vector.json   # the port's own answer to one vector
```

Do not edit a file in this directory. An edit that survives a regeneration is an expectation
nothing produced.

**`regenerate` does not rewrite all of it, and the exceptions are decisions rather than gaps.**
`audio/` is **refused**: the Python detector that recorded it was deleted in M20, and rebuilding
it from the Rust one would be a self-portrait that passes by construction. `stages/` derives from
the committed engine vectors rather than from `data/processed/`, and `format/` from CPython itself,
so `regenerate --stages-only` and `regenerate --format-only` rebuild those two on a machine with no
captures on it — which a full `regenerate` cannot claim, because `synthetic/` and `corpus/` need the
capture machine. `docs/CONFORMANCE.md` §4 has all four.
