"""The committed club catalogue — so the second lookup of a club is offline. [M12 P3]

A specification is stable for the life of a model: a 2023 Titleist T150 7 iron has the same loft
this year, next year and for every golfer who owns one. Asking a model for it twice is therefore
not a second measurement, it is the same answer bought again — and the second purchase is the one
that is slow, costs money and can come back *different*, which is the part that matters. A
catalogue makes the answer a fact of this repo rather than a fact of whichever model was called.

This is [ADR-022](../../../docs/decisions/022-learned-artifacts-as-committed-data.md)'s division
applied to a lookup instead of a fit: something heavier than the package produces the data, the
data is committed as JSON with provenance per row, and what runs inside the package is a dictionary
read. `analysis/benchmarks/ranges.json` and `golfdb_v1.json` are the same shape, and
[ADR-026](../../../docs/decisions/026-club-specification-lookup.md) §7 is the decision.

## The file started empty and grows by use, which is not the same as being seeded

A catalogue is worth committing once it has been checked, so nothing is seeded here: P5's save
route calls `remember` when a golfer *confirms* a specification, and that confirmation is the whole
qualification a row has. Shipping a pre-populated catalogue is deferred in ADR-026 §Deferred for
that reason and not for effort.

`club_catalogue.json` shipped with no rows until 2026-09-01, when M12 P6 walked the page end to end
and the first seven arrived — a 2023 Titleist T150 4i–PW, one golfer's confirmed set. That is what
every row here will be: a club somebody owns, not a catalogue somebody bought. The consequence
worth naming is that the long tail stays *missing* rather than wrong — a lookup for a club nobody
in this repo has confirmed costs an API call every time, and the cache never pretends otherwise.

**A row's `provenance.notes` may talk about clubs other than its own.** `ClubLookupOutcome`
carries one provenance block per *call*, and a set lookup is one call for up to fourteen slots, so
`remember` stores the same note on every row it writes: six of the first seven carry a paragraph
covering the whole 4i–PW request, and the pw row's note discusses the 4i. It is a note and nothing
reads it as data, but it is not per-slot and should not be read as if it were. The fix is per-slot
provenance, which needs a shape `ClubSpec` deliberately does not have, and it belongs with the
per-*field* provenance ADR-026 defers rather than being bolted on here.

## Every refusal here is silent on purpose, and that is not the usual posture

`BagStore` raises when it cannot read a bag, because the bag is the record. **This is a cache.** A
lookup that cannot read the catalogue must cost an API call and never an error, and a `remember`
that cannot write must cost a *future* API call and never the save that triggered it — a golfer
whose club is now correctly in their bag should not see a 500 because a cache file was busy. So
every function here degrades to "no catalogue" rather than raising, and `remember` reports what it
did by returning the key it wrote or `None`.

The one thing that refusal must not do is destroy data. `remember` will not write over a catalogue
it cannot read, for `BagStore._load_for_write`'s reason exactly: a writer that treats an unreadable
file as an empty one replaces every checked row in it with the single row it was asked to add.

Stdlib + pydantic, `contracts` only (ADR-008).
"""

from __future__ import annotations

import json
import os
from pathlib import Path

from pydantic import BaseModel, ValidationError

from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import ClubSpec, SpecProvenance
from golf_coach.contracts.golfer import slugify

_CATALOGUE_FILE = "club_catalogue.json"

#: Bumped when the row shape changes incompatibly. A file declaring anything else is read as *no*
#: catalogue rather than as this shape — a newer file misread under this reader would hand a golfer
#: a specification assembled out of fields that moved, which is the one failure a cache is not
#: allowed to have.
_SCHEMA_VERSION = 1


class CatalogueRow(BaseModel):
    """One remembered specification and where it came from.

    Provenance is **required** here where `BagEntry.provenance` is optional, and the asymmetry is
    deliberate: a bag entry can predate anything that recorded a source, but a row only exists
    because something produced it, and a row that cannot say what would make the catalogue's whole
    claim — "a row that came from an LLM is never mistaken for a row someone typed off the
    manufacturer's page" (ADR-026 §7) — unverifiable one row at a time.

    The provenance travels with the spec rather than being rewritten to `"catalogue"` on the way
    out. The catalogue is not a source; it is where a source's answer was kept, and flattening the
    two would lose exactly the distinction §7 asks it to preserve. `"catalogue"` stays a legal
    `source` for a row typed straight into the file by hand, which is a real provenance and a
    better one than most.

    The key is **not** a field. It is derived from `spec` by `catalogue_key`, so there is one place
    a row's identity is written; a stored key that disagreed with the make and model beside it
    would produce a row that can never be found and that nothing would ever flag.
    """

    spec: ClubSpec
    provenance: SpecProvenance


def catalogue_key(make: str, model: str, model_year: int | None, club: ClubId) -> str:
    """The four identity fields to one key.

    `("Titleist", "T-150", 2023, ClubId.SEVEN_IRON)` becomes `titleist/t150/2023/7i`.

    **Normalised through `slugify` (`contracts/golfer.py:54`), not a second normaliser.** That
    function exists because "Aaron" and "aaron" splitting one golfer's baseline in two is silent
    and undetectable, and "T150" and "T-150" splitting one club into two catalogue rows is the same
    failure with the same invisibility — the catalogue simply misses, calls the model again, and
    stores a second row nobody sees (ADR-026 §7, ADR-024 §1).

    Separators are then dropped, which `slugify` does not do and must not: a `player_id` is a
    filename people read, so "mary jane" and "maryjane" are allowed to be two golfers. A model name
    is never read back out, and "T150", "T-150" and "T 150" are one club printed three ways on the
    same manufacturer's own pages. Dropping the hyphen is the whole difference between those two
    jobs, and it is why this composes `slugify` rather than replacing it.

    **An unknown year is its own key, not a wildcard.** `None` becomes an empty segment and matches
    only other rows that also failed to establish a year. Makers reuse a model name across
    generations with different lofts, so falling back to "any year" would answer a 2023 lookup with
    a 2019 set's numbers and nothing downstream could tell — the same trade `parse_club` refuses
    when it returns `None` rather than a nearest match.

    Blank components are not rejected here. This is a normaliser and refusing is policy, which
    belongs to `lookup` and `remember` — both of which decline a spec with no make or model, since
    a row keyed on emptiness would answer every make-less lookup with whatever was stored last.
    """
    return "/".join(
        (
            slugify(make).replace("-", ""),
            slugify(model).replace("-", ""),
            str(model_year) if model_year is not None else "",
            club.value,
        )
    )


def key_for(spec: ClubSpec) -> str:
    """`catalogue_key` off a spec's own identity fields, so no caller re-lists the four."""
    return catalogue_key(spec.make, spec.model, spec.model_year, spec.club)


def load_catalogue() -> dict[str, CatalogueRow]:
    """Every remembered row, keyed. An absent, unreadable or unknown-version file is an empty one.

    Deliberately **uncached**, where `benchmarks/store.py:_load_ranges` caches its rows for the
    life of the process. Nothing writes `ranges.json` while the server runs; `remember` writes this
    file, and a cache would mean a lookup answering out of the state before the last confirmed
    club. The file holds tens of rows of JSON and is read once per lookup, which is a cost too
    small to trade a staleness bug for.

    **Row-level tolerance, not just file-level.** A row that fails validation is skipped and the
    rest of the catalogue survives, because one hand-edited row with a typo in it is not a reason
    to stop serving the forty that were checked. A file that will not parse at all is empty, which
    is the same statement one level up.
    """
    rows = _read_rows()
    return {} if rows is None else rows


def lookup(make: str, model: str, model_year: int | None, club: ClubId) -> CatalogueRow | None:
    """The remembered specification for one slot of one model, or `None` on a miss.

    Returns the whole row rather than the bare `ClubSpec`: the caller needs the provenance to say
    where the numbers came from, and a spec handed over with its provenance stripped is precisely
    the "row that came from an LLM mistaken for one typed off the manufacturer's page" that
    ADR-026 §7 asks this file to prevent.

    A miss is not an error and is the normal case for the first lookup of any club. P4's model call
    is what a miss routes to.
    """
    if not slugify(make) or not slugify(model):
        return None
    return load_catalogue().get(catalogue_key(make, model, model_year, club))


def remember(spec: ClubSpec, provenance: SpecProvenance) -> str | None:
    """Write an accepted specification into the catalogue. Returns the key written, or `None`.

    Called when a golfer **confirms** a specification, not when one is looked up (ADR-026 §5). What
    the model proposed is a proposal; what a person accepted is worth serving to the next lookup.

    Re-remembering a key replaces its row rather than appending a second one, so the file holds one
    row per club and a corrected spec supersedes the one it corrects. Rows are written in key order
    for the same reason `ranges.json` is readable: this is committed data, and a diff over it is
    how anyone reviews what the catalogue learned.

    Returns `None`, having written nothing, in three cases, all of them silent by design (see the
    module docstring):

    - **the spec has no make or model** — nothing could key it, and a row under the empty key would
      be served for every make-less lookup thereafter;
    - **the catalogue exists and cannot be read** — refusing here is `BagStore._load_for_write`'s
      rule, and the loss it prevents is larger than the one it costs: a tolerant writer would
      replace every checked row with this one;
    - **the write fails** — the catalogue is a cache and the bag is the record, so a full disk
      costs the next lookup an API call and must not cost the golfer the save that got them here.
    """
    if not slugify(spec.make) or not slugify(spec.model):
        return None

    rows = _read_rows()
    if rows is None:
        return None

    key = key_for(spec)
    rows[key] = CatalogueRow(spec=spec, provenance=provenance)
    payload = {
        "schema_version": _SCHEMA_VERSION,
        # `exclude_defaults` because every default here is the blank, so an omitted field and a
        # written `null` make the same statement — and a committed file a human is meant to check
        # reads better when a row shows what is *known* about a club rather than twenty-seven
        # keys, most of them empty.
        "clubs": [rows[k].model_dump(mode="json", exclude_defaults=True) for k in sorted(rows)],
    }

    path = _catalogue_path()
    tmp = path.with_suffix(".tmp")
    try:
        tmp.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        os.replace(tmp, path)
    except OSError:
        tmp.unlink(missing_ok=True)
        return None
    return key


def _catalogue_path() -> Path:
    """The packaged JSON, as a real path.

    `benchmarks/store.py` reads its data through `importlib.resources`, which is the right API for
    a file that is only ever read. This one is **written** at runtime, and a `Traversable` cannot
    be. Naming the file two ways — a resource to read and a path to write — would let the read and
    the write disagree about which catalogue this is, which is worse than the portability a
    filesystem path gives up: an installation that cannot write beside its own modules gets a
    catalogue that never fills, and that is the degraded case this module already handles.
    """
    return Path(__file__).with_name(_CATALOGUE_FILE)


def _read_rows() -> dict[str, CatalogueRow] | None:
    """Parse the catalogue. `None` means "there is a file and it cannot be trusted".

    The distinction is only for `remember`, and it is `BagStore._load_for_write`'s: a reader can
    treat unreadable as empty and lose nothing, while a writer that does the same overwrites
    everything it failed to read. `load_catalogue` collapses the two on the way out.

    An absent file is `{}` and not `None` — no catalogue yet is a state to write into, and it is
    the state this milestone starts in.
    """
    path = _catalogue_path()
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    if not isinstance(payload, dict) or payload.get("schema_version") != _SCHEMA_VERSION:
        return None
    raw = payload.get("clubs")
    if not isinstance(raw, list):
        return None

    rows: dict[str, CatalogueRow] = {}
    for entry in raw:
        try:
            row = CatalogueRow.model_validate(entry)
        except ValidationError:
            continue
        rows[key_for(row.spec)] = row
    return rows
