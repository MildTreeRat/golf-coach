---
description: Reconcile the docs with the code, starting from tests/test_docs_truth.py
argument-hint: "[optional: a doc or claim you already suspect]"
---

Use the Agent tool with `subagent_type: doc-keeper` to reconcile this repo's documentation with its
code.

Extra context from the user (may be empty — if so, run the standard pass): $ARGUMENTS

Pass the agent this instruction:

> Run the doc-truth oracle first, work only from its failures, auto-fix mechanical claims, and
> report anything needing a sentence rewritten. Return the two-table contract and the final pytest
> line. `verdict: clean` is a correct and expected result — do not manufacture findings.

Relay both tables to me verbatim. Do not apply anything from the `NEEDS YOU` table yourself; those
are mine to word.
