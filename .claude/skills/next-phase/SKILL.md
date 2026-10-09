---
name: next-phase
description: Resume a phased plan from docs/plans/ in a fresh session and complete exactly one phase, then stop. Use when the user types /next-phase.
disable-model-invocation: true
---

# Next Phase

Plan (optional): $ARGUMENTS

1. Open the plan in `docs/plans/` (the one named above, or the one with unfinished phases; ask if
   several) — **by its map, not end to end** (*Reading the plan*, below).
2. Find the first unchecked phase: any State that is not ✅. **Mark it started before any other
   work** — set its State cell to `🔄 Started <YYYY-MM-DD HH:MM>`, the time from
   `date '+%Y-%m-%d %H:%M'` (batch that with the map). If the row already reads 🔄, a previous
   attempt was cut off: leave the marker as it is and follow *Resuming*, below.
3. Read only the context that phase needs. Complete that one phase. Verify it against its done
   criteria.
4. Check it off in the plan doc (✅ replaces the 🔄) and append what the phase found to the plan's
   findings. Address anything a later phase must know **to that phase by number** — "**For P12**:
   …" — because that is how the later phase finds it without reading every finding.
5. **Stop.** Do not start the next phase. Reply briefly: what was done, what's next, and suggest
   `/clear` then `/next-phase`.

If something in the plan turns out wrong, stop and ask instead of improvising.

## Reading the plan

Plans here run past 1,500 lines and grow by a findings entry every phase. Read whole, the plan alone
was a sixth of every M36 phase's input tokens, and close to a third by P10. Map it first:

```
grep -n '^## \|^### ' docs/plans/<plan>.md
```

Then read:

- **everything above `## Phases`** — the checklist, the rules, the verify commands, the decisions
  and calls. It is the contract, and it does not grow.
- **your own `### Pn` section.**
- **from `## Phase findings`, only**: the entries your section's Read line names; every line
  addressed to your phase (`grep -n '\bP<n>\b'` below the findings heading, then read the items it
  lands in); and lines naming the files your section lists. A finding that contradicts the plan
  wins, so these greps are not optional — they are the cheap way to be sure none is missed.

A plan without that `## Phases` / `## Phase findings` split is read whole.

## Spending context

Every tool call re-sends your whole context: at 200k, one extra call costs what reading 200k tokens
does, and an M36 phase made 45–95 calls. So:

- Batch independent reads — several tool calls in one message, or one Bash call — rather than one
  probe per turn.
- Read a file once, whole or in the ranges you need, and do not re-read what has not changed.
- Pipe build and test output through `tail` or `grep`.

## Resuming

A 🔄 row means a session started this phase and was cut off — in M36 a usage limit did it twice,
both mid-phase — so its partial work may be in the tree. Before writing anything, list what changed
after the marker's time:

```
find crates src tests scripts spec docs -type f -newermt '<marker time>' -not -path '*/target/*'
```

Read what that lists against your section's Files line: keep what is correct, finish what is not,
and say in the findings that the phase was resumed. Restarting from nothing pays for the same phase
twice.
