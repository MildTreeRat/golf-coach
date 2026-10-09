---
name: orchestrate
description: Run a milestone's remaining phases back to back — one fresh subagent per phase — and stop on a question, a failure, or the phase cap. Use when the user types /orchestrate.
disable-model-invocation: true
---

# Orchestrate a milestone

Milestone or plan (optional), plus flags: $ARGUMENTS

Flags: `--max N` (phases this run, default 8), `--dry-run` (resolve the plan, report what would
run, launch nothing).

You are the **orchestrator**. You do not do phase work — not one edit, not one test run. Your whole
job is: check whether the milestone is done, launch one subagent to do the next phase, read its
one-line verdict, and repeat. Every phase gets a fresh context; yours has to survive all of them,
so the discipline below about what you may read is the skill, not decoration.

## 0. Resolve the plan (once)

```
ls docs/plans/
```

`$ARGUMENTS` may name a milestone (`m23`), a plan file, or nothing. Match case-insensitively on the
milestone token; with nothing named, pick the single plan that has unchecked phases. If zero or
several match, **ask the user** with AskUserQuestion and stop until answered — this is the one
question you are allowed to raise before the loop starts.

Record the plan path. Announce it in one line: plan, phases left, `--max`.

## 1. Read only the checklist

```
sed -n '/^## Status checklist/,/^---$/p' docs/plans/<plan>.md
```

That table is the whole state machine. **Do not read the plan body, the ADRs, the code, or the
diff** — every one of those is the subagent's job, and reading them here is what makes this loop
stop working after three phases instead of ten.

Unchecked rows are the ones whose State is not ✅: `⬜` not started, `🔄 Started …` (a session
began the phase and was cut off — the phase runner marks its row before it starts work), blocked.
The first unchecked row is the next phase. A 🔄 row is a resume, not a fresh start, and the
announcement says so.

## 2. Stop conditions, checked before every launch

Check all of these each time round, and stop at the first that trips:

- **Done** — no unchecked rows. The milestone is complete; report and stop.
- **Compaction** — your context was summarized during this run. One compaction is survivable; a
  second means the loop is costing more than it saves. Stop and say so.
- **Cap** — `--max` phases have been run this session.
- **No progress** — the previous phase reported done but the checklist row is still unchecked (§4).

There is deliberately **no token-budget stop**. The figure in `<total_tokens … left>` counts only
your own context, not your subagents': across five M36 phases it fell from 15.0M to 14.85M while
the phases themselves spent tens of millions. The limit that actually ends a run is the plan's usage
limit, and nothing in your context shows it coming. Both M36 runs were cut off by it mid-phase. The
defense is the 🔄 marker: a cut-off phase resumes from its partial work on the next run instead of
starting over.

On any stop that is not *Done*, the report tells the user exactly how to resume: `/clear`, then
`/orchestrate <milestone>`.

## 3. Launch exactly one subagent

One at a time, sequentially, never in parallel — phase N+1 is planned against what phase N found, so
two in flight would both work from stale findings. Use the Agent tool with `subagent_type:
"phase-runner"` (`.claude/agents/phase-runner.md`: a fresh context, the unattended contract, and a
short tool list). **Not** `fork`, which would inherit your context and defeat the point.

Prompt, filled in:

> Plan: `docs/plans/<plan>.md`. Phase: **<Pn>**.
>
> *(only when the row reads 🔄)* Its row reads `<the State cell, verbatim>` — a previous attempt
> was cut off. Resume it as the next-phase skill's *Resuming* section says.

The contract — follow the next-phase skill, never ask the user, end on a sentinel — lives in the
agent file, not here, so the prompt stays two lines.

**If the Agent tool rejects `phase-runner`** (an agent file added after the session started may not
be registered until Claude Code restarts), launch `general-purpose` with the same prompt and this
first line: *Read `.claude/agents/phase-runner.md` and follow its body as your instructions; it is
your contract.* Say in your log that the run fell back, since that agent carries every tool's schema
in every call.

## 4. Read the verdict, then verify it

Take the sentinel from the first line. Then re-read **only the checklist** (§1) and confirm the row
for `<Pn>` is now ✅.

| Sentinel | Row ticked | Do |
|---|---|---|
| `PHASE_DONE` | yes | log one line, loop to §1 |
| `PHASE_DONE` | no | stop — *No progress*. The agent believes it finished and the plan disagrees; that is exactly the case a human should look at |
| `BLOCKED` | either | **stop the whole run** and bring the question to the user (§5) |
| `FAILED` | either | stop and report the failure verbatim, including the failing command |
| no sentinel | either | treat as `FAILED` — an agent that ignored its contract is not one whose "done" you can trust |

A `BLOCKED` or `FAILED` row stays 🔄, and that is correct: the next run resumes it with its partial
work in view.

Keep your own running log to **one line per phase**. Resist writing a paragraph about a phase you
deliberately did not read.

## 5. When a phase has a question

Cancel orchestration — do not launch the next phase, and do not answer the question yourself; the
plan's decisions are the user's. Present it with AskUserQuestion: the agent's question as asked, its
options as the choices, its recommendation first and marked. Say which phase asked and that the run
is stopped at that boundary.

When the user answers, do **not** resume the loop silently. Record the answer where the next session
will see it — the relevant phase's section in the plan doc — then tell them to `/clear` and re-run
`/orchestrate <milestone>`. A question answered in this context is worth nothing to the fresh
subagent that needs it unless it is written down.

## 6. Final report

Short. What ran (one line per phase), why the loop stopped, what is left in the checklist, and
whether the tree has uncommitted work. **Never commit on your own** — phases commit if their plan
says to, and a milestone-wide commit is the user's call.

## Why this shape

- **The subagents are fresh and you are not.** The whole reason this works is that phase context
  dies with the agent that used it. The moment you start reading plan bodies or diffs "to check",
  you become the thing the phased-plan discipline exists to avoid, and the loop's useful length
  drops from ten phases to three.
- **The checklist is the oracle, not the agent's own report.** Same rule as this repo's conformance
  vectors: the artifact on disk decides, not the process that claims to have produced it.
- **A question stops everything.** A phase that guesses at a decision the user owns produces work
  that looks done and has to be redone — and in a loop, the next phase is then planned against the
  guess. Stopping costs one `/clear`; guessing costs the milestone.
- **The phases are the cost, not you.** Measured over M36's eleven phase runs: the orchestrator used
  under 2% of the tokens. Each phase grew from 40k to 230k–400k context over 45–95 calls, and every
  call re-sends the whole context. So the levers are in the phase runner and the next-phase skill —
  a short tool list, reading the plan by its map, batched reads, resuming rather than restarting —
  and in `/plan-phases`, which keeps phases short. None of them is here.
