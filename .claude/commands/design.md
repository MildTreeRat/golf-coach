---
description: Elaborate an accepted ADR into a proposed architecture under docs/proposals/
argument-hint: "docs/decisions/<nnn>-<name>.md"
---

Use the Agent tool with `subagent_type: architect-design`.

Source ADR: $ARGUMENTS

If that is empty or is not a path under `docs/decisions/`, ask which ADR before spawning anything.
This agent's determinism comes from having one specific file as input; a guessed scope loses it.

Pass the agent this instruction:

> Read the ADR in full including its addenda, then produce the nine-section proposal at
> `docs/proposals/<nnn>-<slug>.md`. Write nowhere else. Blast radius must carry real paths and the
> mirrored test file for each. Rejected must include the simpler option and the option of not
> building it. Open questions must be honest — say what would have to be true.

Report back the path, the five-line summary, the Open questions in full, and the invariants line.
Do not paste the document into this conversation — I will read it on disk.
