---
name: next-phase
description: Resume a phased plan from docs/plans/ in a fresh session and complete exactly one phase, then stop. Use when the user types /next-phase.
disable-model-invocation: true
---

# Next Phase

Plan (optional): $ARGUMENTS

1. Open the plan in `docs/plans/` (the one named above, or the one with unfinished phases; ask if several).
2. Find the first unchecked phase. Read only the context that phase needs.
3. Complete that one phase. Verify it against its done criteria.
4. Check it off in the plan doc and add any notes the next session needs.
5. **Stop.** Do not start the next phase. Reply briefly: what was done, what's next, and suggest `/clear` then `/next-phase`.

If something in the plan turns out wrong, stop and ask instead of improvising.
