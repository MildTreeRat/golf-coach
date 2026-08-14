# Refactor ledger

> **Tier: AS-BUILT.** Every row is a refactor that was actually considered against
> [CODE_STANDARDS.md](CODE_STANDARDS.md) and actually decided. Append-only: nothing here is edited
> or removed, only superseded by a later row.

## What this is for

`architect-refactor` reads this file **before** it reads any code, and **may not re-raise anything
recorded here.** That is the whole mechanism.

You cannot make a model deterministic. You can make prior decisions authoritative. Run the review
twice on the same clean module and get "clean" twice — not because the model felt consistent, but
because the ledger already answered. Without it, every run re-discovers the same declined idea and
the reviewer becomes something you learn to ignore, which is the failure mode that makes a review
tool worthless.

This repo already invented the mechanism, for experiments: *"Rejected alternatives stay runnable…
it's why the same idea doesn't get re-litigated"* (`docs/README.md`, Conventions). Same cure,
applied to code review.

## How to add a row

One line, at the **bottom** of the table. Declining should be cheap — if it costs a paragraph,
nobody does it and the ledger stops working.

| Field | What goes in it |
|---|---|
| **Date** | The day it was decided. |
| **Rule** | The rule ID from `CODE_STANDARDS.md`. `—` if the finding cited no rule, which is itself worth recording. |
| **Location** | `file` or `file:symbol`. Not a line number — those move and the row does not. |
| **Decision** | `Declined` (not doing it), `Deferred` (doing it, not now — say what unblocks it), or `Done` (landed; row kept so it is not re-raised against the old shape). |
| **Reason** | One sentence. The *reason*, not the restatement. |

**A `Declined` row is not permanent.** If circumstances change, add a new row that supersedes it —
same location, later date, new decision. The history stays readable and the agent honours the
latest row.

**Scope:** the ledger suppresses a finding *at a location*. It does not switch a rule off
repo-wide. A new instance of the same rule elsewhere is a fresh finding.

---

## Ledger

| Date | Rule | Location | Decision | Reason |
|---|---|---|---|---|
| 2026-08-11 | R1 | `storage/corpus.py`, `mcp/query.py` → `api.state` | Deferred | Both import upward for the tolerant artifact readers rather than keeping three copies of a parser this repo has already been bitten by duplicating. The graph stays acyclic and nothing is broken today. Fix is written down in ADR-008's addendum — move the readers into `storage/analysis_io.py` and re-export — and is unblocked whenever someone wants a commit that does only that. Doing it inside an unrelated change is how a small move becomes an unreviewable diff. |
| 2026-08-13 | R5 | `contracts/checkpoints.py:CHECKPOINT_REGISTRY` + `analysis/checkpoints/CHECKPOINT_EVALUATORS` | Declined | They are keyed by the same names on purpose. ADR-008 forbids `contracts` importing `analysis`, so identity lives in one and behaviour in the other; a test pins them to each other. Merging them would trade a pinned duplication for a broken dependency rule. |
| 2026-08-13 | R11 | `capture/source.py:VideoSource`, `launch_monitor/screen/recognizer.py:TextRecognizer` | Declined | Protocols with one live adapter each, which is what ADR-007 asked for: the hardware and software tracks run in parallel, so the seam exists before the second adapter does. Both name that second in their own docstring. `ShotDataSource` is the precedent that the bet pays — same shape, now four adapters. |
| 2026-08-13 | R11 | `merge.py` (unbuilt), `analysis/checkpoints/outcome.py` (unbuilt), SQLite storage | Declined | Named as seams and deliberately not built — `docs/archive/M4_POC_PLAN.md`'s guardrails. Pose-only meant one stream with nothing to align, and outcome checkpoints need launch-monitor data. `analysis/checkpoints/__init__.py` records the absence of `outcome.py` in its own docstring, which is the right way to leave a seam. Revisit when a second stream or shot data is actually joined per swing, not before. |
| 2026-08-13 | — | `api/static/*.html` | Declined | Hand-written HTML with no framework and no build step is the choice, not an omission. A local-first tool that runs from `pip install -e .` does not get a node toolchain to keep working. |
| 2026-08-13 | R1 | `mcp/query.py`, `mcp/career.py` → `analysis`, `storage`, `launch_monitor` | Declined | `mcp` is a second imperative shell, the same shape as `api` and for the reason ADR-006's 2026-08-10 addendum gives: a tool set re-scoped to swings has to read what analysis wrote. Direction is downward, graph stays acyclic, `analysis` stays pure. ADR-008's 2026-08-13 addendum now says so, because the Dependency rule naming only `api` is what made two review passes re-derive this. |
| 2026-08-13 | R4 | `mcp/query.py:SwingView.measurements` | Done | Said "the three entries in `checkpoints`" while six shipped — the M6.5 bug in a second channel, since a pydantic `description=` ships inside the tool's `outputSchema`. Now interpolates `caveats.ONLY_CHECKPOINTS_ARE_JUDGED`; `tests/test_docs_truth.py` scans every MCP field description for a count claim that disagrees with the registry. |
| 2026-08-13 | R4 | `mcp/query.py:_METRIC_FIELDS` | Done | A hand-typed copy of `ShotData` read via `getattr(..., None)`, so a field added by the R10 adapter would have gone missing from every payload with mypy clean. Kept as a list rather than derived — the metric/non-metric split is a real editorial choice — but `_NON_METRIC_FIELDS` now makes it exhaustive and `tests/mcp/test_query.py` pins it against the model. |
| 2026-08-13 | R1 | `pose/estimator.py`, `pose/side_by_side.py`, `detection/detector.py` → `capture.source.Frame` | Done | Annotation-only uses that were runtime imports, leaving two modules' documented import-cheapness resting on `capture` keeping numpy inside `TYPE_CHECKING`. Now type-only, pinned by `tests/api/test_pipeline_imports.py`. The type-only edge stays: `Frame` holds a numpy image and the R2 rule keeps that out of `contracts`. |
| 2026-08-13 | R13 | `mcp/query.py:_status_of` | Done | Deferred `storage.manifest` to function scope while importing `storage.bundle_store` at module scope, which already loads it — so it broke no cycle, guarded no extra and saved no import, but read as "there is a cycle this way" in the exact import block the row above schedules work in. Moved to module scope. |
