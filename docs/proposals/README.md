# Proposals

> **Tier: TARGET.** Drafts, not decisions. Nothing here has been reviewed, and nothing here is
> routed to from the documentation map.

`architect-design` writes proposed architectures here — one per ADR, elaborated from a decision
that has already been made. Invoked with `/design docs/decisions/<nnn>-<name>.md`.

**Why they land here rather than in `docs/`.** `docs/README.md`'s whole premise is that everything
it routes to is trustworthy: a tier banner tells you how hard to read, and AS-BUILT means trust the
numbers. Unreviewed machine output cannot sit under that promise. A proposal is held as a draft
until it is read; then it is promoted by hand into `docs/` as a TARGET-tier milestone doc, given a
row in the map, and — from that point — covered by `tests/test_docs_truth.py` like everything else.

A proposal that is never promoted is not a failure. Leaving it here, with the reason it was not
taken up, is the same convention as the rest of this repo: rejected alternatives stay where they
can be found.
