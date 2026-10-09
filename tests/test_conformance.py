"""The specification is only a specification while it still describes the code. [M19]

`spec/` is committed data — schemas exported from `contracts/`, and inputs paired with the outputs
this engine produces for them. Committed data goes stale silently, and this one goes stale in the
worst possible way: a Rust port that passes `conformance.py check` against vectors recorded by an
older engine is a port that has been *certified against the wrong answers*. So three things are
pinned here.

1. **The schemas regenerate to the committed bytes**, so a change to a model that nobody exported
   fails at the commit rather than at the port.
2. **Every vector still conforms**, which is `check` run in-process — the same comparison, so there
   is no second set of rules to keep in step.
3. **The serialization a port is asked to match is the one the pipeline actually writes**, which is
   the pin with the least obvious need and the most obvious failure: the `exclude=` set lives at a
   call site in `api/pipeline.py`, not on the contract, so nothing but this would notice the two
   drifting apart.

The comparison rules themselves get unit tests below, because `compare_results` is the part of the
spec a port is judged by and "it passed" from a comparator that cannot see a difference is worth
nothing. The `None`-is-not-zero case is the one to read first: ADR-010 §2 is the rule this whole
repo is built around and a lenient comparator is how a port would be allowed to break it.

**From M32 the second pin is the freeze, not the oracle** (ADR-035 clauses 3 and 4). Rust
re-records the engine and stage families and `cargo test` certifies them; what is pinned here is
that frozen Python still reproduces every value it recorded, compared in
`conformance.frozen_view` with the paths a Rust re-record owns taken out. The three schema roots
Rust owns are pinned by `crates/contracts/tests/schemas.rs`, and skipped by the first pin.
"""

from __future__ import annotations

import functools
import json
import math
import re
import sys
import types
from collections.abc import Sequence
from pathlib import Path
from typing import Any

import pytest

from golf_coach.contracts.swing import ANALYSIS_VERSION

REPO = Path(__file__).resolve().parent.parent
SPEC = REPO / "spec"

# `scripts/` is not a package and not installed — the same convention `scripts/golfdb/common.py`
# documents, which is why this insert is here rather than in `pyproject.toml`.
sys.path.insert(0, str(REPO / "scripts"))

import conformance  # noqa: E402


def _vector_ids() -> list[str]:
    return [conformance._vector_id(p) for p in conformance.engine_vector_paths()]


# --------------------------------------------------------------------------- P1: the schemas


def test_every_committed_schema_is_what_contracts_exports_today() -> None:
    """Regenerate in memory and compare. This is the whole freshness check.

    It fails on a field added to `SwingResult`, on a description reworded, and on a validator that
    changes a bound — all of which are things a port has to be told about, and none of which any
    other test in this repo can see.

    **Not for `RUST_OWNED_SCHEMAS`.** From M32 those three files are edited by hand to Rust's
    wider shape, which frozen Python's models never gain, so this export is no longer what they
    should hold. `crates/contracts/tests/schemas.rs` pins them against the Rust structs instead.
    """
    stale = []
    for name, expected in conformance.export_schemas().items():
        if name in conformance.RUST_OWNED_SCHEMAS:
            continue
        path = conformance.SCHEMAS / f"{name}.schema.json"
        if not path.exists():
            stale.append(f"{name}: never exported")
        elif path.read_text(encoding="utf-8") != expected:
            stale.append(f"{name}: differs from contracts/")

    assert not stale, (
        "spec/schemas is out of date with contracts/ — run "
        "`python scripts/conformance.py regenerate --schemas-only`:\n  " + "\n  ".join(stale)
    )


#: Every `*.json` a `golf_coach` module names as a filename constant, split by whether a Rust core
#: has to *parse* it. The split is the rule `SCHEMA_ROOTS` applies, made explicit so a new artifact
#: has to be classified rather than forgotten.
#:
#: The exempt half is package data: committed, provenanced JSON that ships *inside* the wheel and
#: ports as bytes (ADR-022). A Rust core loads `ranges.json` the way Python does — as numbers to
#: evaluate, not as a contract two implementations must agree on the shape of.
_PACKAGE_DATA = {
    "ranges.json",  # ADR-010/022: the benchmark bands
    "golfdb_v1.json",  # ADR-012: the tour distributions
    "joint_model_v1.json",  # ADR-022: the first learned artifact
    "flight_model_v1.json",  # ADR-027: the drag/lift coefficient table
    "club_catalogue.json",  # ADR-026: the committed specification dictionary
    # ADR-014: the frozen screen parser's device profiles. `crates/screen` forks it rather than
    # reading it (M34), so this copy lacks the `Impact Position V` tile, and M40 deletes it.
    "profiles.json",
}

#: artifact filename -> the `SCHEMA_ROOTS` key that describes it.
_ARTIFACT_SCHEMAS = {
    "manifest.json": "swing_manifest",
    "session.json": "session_meta",
    "analysis.json": "swing_bundle_result",
    "analysis.state.json": "analysis_state",
    ".bag.json": "bag",
    ".golfer.json": "golfer",
    ".shot.json": "shot_data",
    # No filename constant — these are built from the clip's role at the call site.
    "{role}.keypoints.json": "keypoints_file",
    "{role}.audio.json": "audio_file",
}


def test_every_on_disk_artifact_has_a_schema() -> None:
    """The rule `SCHEMA_ROOTS` applies, checked rather than trusted.

    This test exists because the first pass at that list got it wrong. `SwingManifest` was dropped
    as "internal to Python" — but ADR-030 §1 gives Rust **storage**, so a Rust core opens
    `manifest.json` on the way to every swing, and would have had to reverse-engineer the shape
    from an example file. Three more went the same way: `session.json`, the golfer registry and
    the bag, the last two being the files behind the `handedness` and `loft_deg` arguments
    `analysis` is forbidden to fetch for itself.

    Discovery over a listing, so a *new* artifact fails here: the constants are scraped out of the
    source, and each one must be either exempt package data or mapped to a schema root.
    """
    constants = set()
    for path in (REPO / "src" / "golf_coach").rglob("*.py"):
        for match in re.finditer(
            r'^_?[A-Z][A-Z_]* = "([^"]*\.json)"', path.read_text(encoding="utf-8"), re.MULTILINE
        ):
            constants.add(match.group(1))

    unclassified = constants - _PACKAGE_DATA - set(_ARTIFACT_SCHEMAS)
    assert not unclassified, (
        f"{sorted(unclassified)} is a JSON artifact nothing here has classified. Either it ships "
        "inside the wheel as data (add it to _PACKAGE_DATA with its ADR) or a non-Python "
        "implementation has to parse it (give it a SCHEMA_ROOTS entry and map it here)."
    )

    missing = {
        name: root
        for name, root in _ARTIFACT_SCHEMAS.items()
        if root not in conformance.SCHEMA_ROOTS
    }
    assert not missing, f"these artifacts map to schema roots that do not exist: {missing}"


def test_no_schema_is_committed_without_a_root_that_produces_it() -> None:
    """The other direction: a file left behind by a root that was renamed or dropped.

    A stale schema is worse than a missing one, because a port reads it and implements a shape
    this repo no longer has.
    """
    committed = {p.name.removesuffix(".schema.json") for p in conformance.SCHEMAS.glob("*.json")}
    assert committed == set(conformance.SCHEMA_ROOTS), (
        f"spec/schemas holds {sorted(committed - set(conformance.SCHEMA_ROOTS))} that "
        f"SCHEMA_ROOTS does not produce"
    )


def test_schemas_only_writes_the_python_owned_roots_and_none_of_rusts(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """`regenerate --schemas-only` must not write frozen Python's shape over a Rust-owned file.

    Written into a temporary directory, never `spec/`. The subset check is the typo guard: a
    misspelled name in `RUST_OWNED_SCHEMAS` would protect nothing and skip nothing, and the
    freshness test above would still pass on it.
    """
    assert conformance.RUST_OWNED_SCHEMAS <= set(conformance.SCHEMA_ROOTS)
    monkeypatch.setattr(conformance, "SCHEMAS", tmp_path)
    written = {p.name.removesuffix(".schema.json") for p in conformance.write_schemas()}
    assert written == set(conformance.SCHEMA_ROOTS) - conformance.RUST_OWNED_SCHEMAS
    assert {p.name for p in tmp_path.iterdir()} == {f"{name}.schema.json" for name in written}


# --------------------------------------------------------------------------- P2: the vectors


def test_there_are_vectors_at_all() -> None:
    """A regeneration that silently produced nothing would make every test below vacuous."""
    ids = _vector_ids()
    assert any(i.startswith("synthetic/") for i in ids), "no synthetic vectors"
    assert any(i.startswith("corpus/") for i in ids), "no corpus vectors"


@pytest.mark.parametrize("path", conformance.engine_vector_paths(), ids=conformance._vector_id)
def test_each_vector_still_conforms(path: Path) -> None:
    """The freeze, checked: frozen Python still reproduces every value it recorded.

    `conformance.py check` runs the same comparison. Both compare in `frozen_view`, which takes
    the paths a Rust re-record owns out of both sides (M32). On a vector with no ledger that is the
    whole answer, which is the comparison this test made before M32.

    A failure here means frozen Python moved, outside the paths Rust declared. The lab is frozen
    from M32 (ADR-035 clause 4), so that is either a fix to something that broke, which should
    say so, or a change nobody meant. It never means "re-record": Rust re-records, against Rust.
    """
    vector = conformance._read_json(path)
    expected, actual = conformance.frozen_view(vector, conformance.run_vector(vector))
    diffs = conformance.compare_results(expected, actual)
    assert not diffs, "\n".join(["frozen Python disagrees with the committed vector:"] + [
        f"  {d}" for d in diffs[:10]
    ])


@pytest.mark.parametrize("path", conformance.engine_vector_paths(), ids=conformance._vector_id)
def test_each_vector_is_at_the_frozen_version_or_ledgered_above_it(path: Path) -> None:
    """A vector frozen Python can be held to: at its version, or above it with a ledger.

    Before M32 this pinned equality with `ANALYSIS_VERSION`. From M32, Rust re-records, and the
    version moves past frozen Python's on purpose (ADR-035 clauses 3 and 4). So above it, the
    vector needs a `provenance.rerecords` entry for each version between, saying which values that
    version moved. Without one, `frozen_view` cannot tell Rust's values from Python's. "Recorded at
    the current version" is still pinned, against Rust's constant, in `crates/core/tests/engine.rs`.
    """
    vector = conformance._read_json(path)
    assert conformance.ledger_covers(vector, ANALYSIS_VERSION), (
        f"{path.name}: {conformance._staleness(vector)}"
    )


@pytest.mark.parametrize("path", conformance.engine_vector_paths(), ids=conformance._vector_id)
def test_each_vector_carries_its_provenance(path: Path) -> None:
    """Where a vector came from is the difference between evidence and a plausible file.

    `ranges.json` carries provenance per row for this reason (ADR-022) and so does
    `club_catalogue.json`; a golden vector is the same kind of committed data and gets the same
    rule.
    """
    prov = conformance._read_json(path).get("provenance", {})
    assert prov.get("kind") in {"synthetic", "corpus"}, f"{path.name} has no provenance kind"
    assert prov.get("note"), f"{path.name} does not say where it came from"


def test_the_corpus_vectors_record_the_offset_that_maps_them_back() -> None:
    """A corpus vector is a *slice*, so its frame indices are not the archive's.

    Without `frame_offset` there is no way to check one against the swing directory it came from,
    and the vectors become fifteen files that agree only with themselves.
    """
    for path in conformance.engine_vector_paths():
        vector = conformance._read_json(path)
        if vector["provenance"]["kind"] != "corpus":
            continue
        offsets = vector["provenance"].get("frame_offset")
        assert offsets and "face_on" in offsets, f"{path.name} records no face_on frame_offset"


# --------------------------------------------------------------------------- P3: the rules


def test_a_refusal_never_compares_equal_to_a_zero() -> None:
    """ADR-010 §2 at the comparator. The single most important assertion in this file.

    A port that returns 0.0 where this engine returns `None` has turned "could not measure" into
    "measured zero" — the exact failure the unscored list exists to prevent — and a comparator
    that treated them as close enough would certify it.
    """
    assert conformance.compare_results({"score": None}, {"score": 0.0})
    assert conformance.compare_results({"score": None}, {"score": 0})
    assert conformance.compare_results({"score": 0.0}, {"score": None})
    assert not conformance.compare_results({"score": None}, {"score": None})


def test_a_verdict_is_compared_as_a_verdict_and_not_as_a_number() -> None:
    """`isinstance(True, int)` is true in Python, so the obvious comparator gets this wrong.

    `passed` is the field a golfer is actually told about. Comparing it numerically would let
    `1` through for `True` — harmless in Python, and a genuine shape difference in a port's JSON.
    """
    assert conformance.compare_results({"passed": True}, {"passed": 1})
    assert conformance.compare_results({"passed": False}, {"passed": 0})
    assert not conformance.compare_results({"passed": True}, {"passed": True})


def test_floats_admit_reassociation_and_nothing_wider() -> None:
    """The tolerance is sized for a different summation order, not for a different rule.

    A hundredth of a degree is a different measurement and must fail; the last bits of an f64 are
    the same measurement added up in a different order and must pass.
    """
    assert not conformance.compare_results({"v": 12.340000000000001}, {"v": 12.34})
    assert conformance.compare_results({"v": 12.34}, {"v": 12.35})
    assert conformance.compare_results({"v": 12.34}, {"v": 12.3400001})


def test_structure_is_exact() -> None:
    """Same keys, same lengths, same order — a missing checkpoint is not a small difference."""
    assert conformance.compare_results({"a": 1}, {"a": 1, "b": 2})
    assert conformance.compare_results({"a": 1, "b": 2}, {"a": 1})
    assert conformance.compare_results([1, 2, 3], [1, 2])
    assert conformance.compare_results([1, 2], [2, 1])
    assert not conformance.compare_results([1, 2], [1, 2])


def test_a_difference_names_the_path_a_reader_can_find_it_at() -> None:
    """A failure a port author cannot locate in the file is a failure they cannot fix."""
    diffs = conformance.compare_results(
        {"swing": {"checkpoint_scores": [{"score": 80.0}]}},
        {"swing": {"checkpoint_scores": [{"score": 60.0}]}},
    )
    assert [d.path for d in diffs] == [".swing.checkpoint_scores[0].score"]


# --------------------------------------------------------------------------- M32 P6: the freeze
#
# Every test here runs on an in-memory vector. No committed vector carries a `rerecords` ledger
# until M32 P8, so on the real files `frozen_view` takes nothing out and `ledger_covers` is plain
# equality. That makes these fixtures the only thing that reaches the new branches before P8, and
# the real suite passing beside them is the proof the view changes nothing today.

_ABOVE = ANALYSIS_VERSION + 1


def _entry(
    version: int, *, added: Sequence[str] = (), moved: Sequence[str] = ()
) -> dict[str, Any]:
    """One `provenance.rerecords` entry, in the shape `golf-core rerecord` writes."""
    return {
        "analysis_version": version,
        "by": "golf-core rerecord",
        "declaration": f"spec/declarations/v{version}.json",
        "added": list(added),
        "moved": list(moved),
    }


def _rerecorded(*, carry: float = 150.0) -> tuple[dict[str, Any], dict[str, Any]]:
    """A vector Rust re-recorded one version above frozen Python, and frozen Python's answer.

    Shaped like M32's own re-record: the version moved at both of its paths, and a shot key added
    that frozen Python's `ShotData` does not have. `carry` is frozen Python's carry, so a test can
    move a value nobody declared.
    """
    vector = {
        "analysis_version": _ABOVE,
        "provenance": {
            "kind": "corpus",
            "oracle": "python",
            "rerecords": [
                _entry(
                    _ABOVE,
                    added=["expected.swing.shot.attack_angle"],
                    moved=["analysis_version", "expected.analysis_version"],
                )
            ],
        },
        "expected": {
            "analysis_version": _ABOVE,
            "swing": {"shot": {"carry": 150.0, "attack_angle": -3.1}},
        },
    }
    actual = {"analysis_version": ANALYSIS_VERSION, "swing": {"shot": {"carry": carry}}}
    return vector, actual


def test_the_frozen_view_takes_out_what_rust_declared() -> None:
    """A declared move and a declared added key are Rust's, so frozen Python is not held to them."""
    vector, actual = _rerecorded()
    assert conformance.compare_results(vector["expected"], actual), "the fixture differs at all"
    assert not conformance.compare_results(*conformance.frozen_view(vector, actual))


def test_the_frozen_view_still_sees_a_value_nobody_declared() -> None:
    """The freeze is only checked while everything undeclared is still compared."""
    vector, actual = _rerecorded(carry=151.0)
    diffs = conformance.compare_results(*conformance.frozen_view(vector, actual))
    assert [d.path for d in diffs] == [".swing.shot.carry"]


def test_the_frozen_view_edits_neither_the_vector_nor_the_answer_it_views() -> None:
    """A view that removed paths in place would hide them from every later check of the vector."""
    vector, actual = _rerecorded()
    before = json.dumps([vector, actual], sort_keys=True)
    conformance.frozen_view(vector, actual)
    assert json.dumps([vector, actual], sort_keys=True) == before


def test_without_a_ledger_the_frozen_view_is_the_plain_comparison() -> None:
    """Before M32's re-record, which is every vector until P8, the view must change nothing."""
    vector = {
        "analysis_version": ANALYSIS_VERSION,
        "provenance": {"kind": "synthetic"},
        "expected": {"analysis_version": ANALYSIS_VERSION, "swing": {"score": 80.0}},
    }
    actual = {"analysis_version": ANALYSIS_VERSION, "swing": {"score": 60.0}}
    expected, viewed = conformance.frozen_view(vector, actual)
    assert expected is vector["expected"] and viewed is actual
    assert [d.path for d in conformance.compare_results(expected, viewed)] == [".swing.score"]


def test_a_stage_vector_is_viewed_through_its_stages() -> None:
    """A stage vector's answer is `stages`, so a ledger path under it comes out of `stages`."""
    vector = {
        "analysis_version": _ABOVE,
        "provenance": {
            "kind": "stages",
            "rerecords": [
                _entry(_ABOVE, moved=["analysis_version", "stages.measure[1].value"]),
            ],
        },
        "stages": {"measure": [{"value": 1.0}, {"value": 2.0}, {"value": 3.0}]},
    }
    actual = {"measure": [{"value": 1.0}, {"value": 2.5}, {"value": 3.0}]}
    assert not conformance.compare_results(*conformance.frozen_view(vector, actual))


def test_a_declared_list_element_is_blanked_and_the_rest_stay_aligned() -> None:
    """Popping a declared element would shift every later one onto the wrong partner."""
    vector = {
        "analysis_version": _ABOVE,
        "provenance": {"kind": "corpus", "rerecords": [_entry(_ABOVE, moved=["expected.v[0]"])]},
        "expected": {"v": [9.0, 2.0, 3.0]},
    }
    assert not conformance.compare_results(
        *conformance.frozen_view(vector, {"v": [1.0, 2.0, 3.0]})
    )
    diffs = conformance.compare_results(*conformance.frozen_view(vector, {"v": [1.0, 2.0, 4.0]}))
    assert [d.path for d in diffs] == [".v[2]"]


@pytest.mark.parametrize(
    ("text", "steps"),
    [
        ("analysis_version", ["analysis_version"]),
        ("expected.swing.shot.attack_angle", ["expected", "swing", "shot", "attack_angle"]),
        ("a.b[1].c", ["a", "b", 1, "c"]),
        ("a[0][10]", ["a", 0, 10]),
    ],
)
def test_a_ledger_path_parses_in_rusts_spelling(text: str, steps: list[str | int]) -> None:
    assert conformance.parse_ledger_path(text) == steps


@pytest.mark.parametrize(
    "text", ["", ".analysis_version", "a[*].b", "a..b", "a.", "a[01]", "a[+1]", "[0]", "a[1]b"]
)
def test_a_ledger_path_rust_would_refuse_is_refused(text: str) -> None:
    """The grammar of `crates/core/src/rerecord.rs::LedgerPath::parse`, and nothing looser.

    A ledger path that parsed here and not there, or there and not here, would be a path one
    language takes out of the comparison and the other does not. The leading dot is the one most
    worth refusing: it is `compare_results`' spelling, and the obvious thing to write by hand.
    """
    with pytest.raises(ValueError, match="not a ledger path"):
        conformance.parse_ledger_path(text)


def test_ledger_covers_a_version_only_where_the_ledger_says_so() -> None:
    """At frozen Python's version as before; above it, only with an entry for every step."""
    at = {"analysis_version": ANALYSIS_VERSION, "provenance": {"kind": "corpus"}}
    assert conformance.ledger_covers(at, ANALYSIS_VERSION)

    unledgered = {"analysis_version": _ABOVE, "provenance": {"kind": "corpus"}}
    assert not conformance.ledger_covers(unledgered, ANALYSIS_VERSION)

    ledgered = {
        "analysis_version": _ABOVE,
        "provenance": {"kind": "corpus", "rerecords": [_entry(_ABOVE, moved=["analysis_version"])]},
    }
    assert conformance.ledger_covers(ledgered, ANALYSIS_VERSION)

    # Two versions above, with the ledger for only the first: the second moved values nobody
    # wrote down, so the view would have nothing to take them out by.
    skipped = {**ledgered, "analysis_version": _ABOVE + 1}
    assert not conformance.ledger_covers(skipped, ANALYSIS_VERSION)

    below = {"analysis_version": ANALYSIS_VERSION - 1, "provenance": {"kind": "corpus"}}
    assert not conformance.ledger_covers(below, ANALYSIS_VERSION)
    assert "golf-core rerecord" in (conformance._staleness(below) or "")
    assert "golf-core rerecord" in (conformance._staleness(unledgered) or "")


def _refuse_to_build(monkeypatch: pytest.MonkeyPatch) -> list[str]:
    """Stand every builder and writer `regenerate` could reach in for a recorder.

    So a refusal that regressed would record a call here rather than read `data/processed/` or
    write over `spec/`. The fake `conformance_vectors` matters as much as the writers: the real
    `build_all` reads the capture machine's archive.
    """
    calls: list[str] = []

    def recorder(name: str) -> object:
        def record(*args: object, **kwargs: object) -> list[object]:
            calls.append(name)
            return []

        return record

    fake = types.ModuleType("conformance_vectors")
    for name in (
        "build_all",
        "build_stages",
        "build_stages_from_disk",
        "build_format",
        "build_screen",
        "build_storage",
        "build_career",
    ):
        setattr(fake, name, recorder(name))
    monkeypatch.setitem(sys.modules, "conformance_vectors", fake)
    monkeypatch.setattr(conformance, "_write_json", recorder("_write_json"))
    monkeypatch.setattr(conformance, "write_schemas", recorder("write_schemas"))
    return calls


@pytest.mark.parametrize("argv", [["regenerate"], ["regenerate", "--stages-only"]])
def test_regenerate_refuses_the_families_rust_records(
    argv: list[str], monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """Frozen Python would write its answers over Rust's, and drop the ledger (ADR-035 clause 3).

    Refused on `conformance_vectors._audio`'s precedent, and refused before anything runs: the
    full rebuild does not get to write the schemas first and then stop.
    """
    calls = _refuse_to_build(monkeypatch)
    assert conformance.main(argv) == 2
    assert not calls, f"a refused `regenerate` still reached {calls}"
    message = capsys.readouterr().err
    assert "golf-core rerecord" in message
    assert "ADR-035 clause 3" in message


def test_regenerate_still_does_the_two_jobs_that_are_pythons(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """The refusal is for the engine and stage families only, and must not catch these two."""
    calls = _refuse_to_build(monkeypatch)
    assert conformance.main(["regenerate", "--schemas-only"]) == 0
    assert conformance.main(["regenerate", "--format-only"]) == 0
    assert calls == ["write_schemas", "build_format"]


# --------------------------------------------------------------------------- the seam itself


def test_the_spec_serializes_a_result_exactly_as_the_pipeline_stores_one() -> None:
    """`EXCLUDED_FROM_RESULT` against the `exclude=` literal in `api/pipeline.py`.

    The exclusion is a *call-site* decision — `model_dump_json(exclude={"swing": {...}})` — so it
    is not on the contract, not in the schema, and invisible to every other test here. A port told
    to match `SwingBundleResult` faithfully would emit the whole keypoint list and differ on a
    field nobody meant to compare; a port told to match this drops the same two fields the results
    page never sees. Read out of the source rather than imported because it is a literal inside a
    function body, and there is nothing else to import.
    """
    source = (REPO / "src" / "golf_coach" / "api" / "pipeline.py").read_text(encoding="utf-8")
    match = re.search(r"exclude=\{\"swing\": \{([^}]*)\}\}", source)
    assert match, "the exclude= literal in api/pipeline.py has moved — re-point this pin"

    excluded = set(re.findall(r'"(\w+)"', match.group(1)))
    assert excluded == conformance.EXCLUDED_FROM_RESULT["swing"], (
        f"api/pipeline.py excludes {sorted(excluded)} when writing analysis.json, but "
        f"conformance.EXCLUDED_FROM_RESULT says "
        f"{sorted(conformance.EXCLUDED_FROM_RESULT['swing'])}"
    )


def test_a_vector_round_trips_through_the_stdin_seam() -> None:
    """`run` is how a non-Python implementation is diffed, so its JSON has to survive the trip.

    In-process rather than through a subprocess: what is being checked is that the serialized
    result parses back and still conforms, not that argparse works.

    Compared in `frozen_view`, like the vectors themselves (M32's plan, call 2): what crosses the
    seam is frozen Python's answer, and a re-recorded vector's declared paths are Rust's.
    """
    path = next(p for p in conformance.engine_vector_paths() if "synthetic" in p.as_posix())
    vector = conformance._read_json(path)
    produced = json.loads(json.dumps(conformance.run_vector(vector)))
    assert not conformance.compare_results(*conformance.frozen_view(vector, produced))


@pytest.mark.parametrize("path", conformance.engine_vector_paths(), ids=conformance._vector_id)
def test_no_vector_carries_an_llm_call(path: Path) -> None:
    """`FeedbackPayload.coaching` is a model's prose, and a model is not reproducible.

    The ranking in `feedback/rules.py` is deterministic stdlib and is pinned; the paragraph
    `feedback/coach.py` asks `claude-opus-5` for is neither. A vector that captured one would fail
    on every run for a reason that has nothing to do with the port — and would put a paid API call
    inside `regenerate`.
    """
    payload = conformance._read_json(path)["expected"].get("feedback") or {}
    assert payload.get("coaching") is None, f"{path.name} recorded an LLM coaching response"
    assert payload.get("coaching_text") is None, f"{path.name} recorded LLM coaching text"


def test_every_vector_carries_the_ranked_feedback_a_results_page_renders() -> None:
    """The half of the artifact a golfer actually reads, and the half a bare engine call omits.

    `analyze_swing_bundle` leaves `feedback` None because `analysis` may not import `feedback`
    (ADR-008); `api/pipeline.py` fills it in a line later. Vectors built off the engine alone
    pinned `"feedback": null` on all of them, which is a specification telling a port to ship no
    coaching. This is what stops that coming back.
    """
    bare = [
        conformance._vector_id(p)
        for p in conformance.engine_vector_paths()
        if not (conformance._read_json(p)["expected"].get("feedback") or {}).get("tips")
    ]
    assert not bare, f"these vectors carry no ranked tips: {bare}"


def test_the_vectors_cover_a_refusal_and_not_only_a_pass() -> None:
    """A suite of passing swings certifies a port that cannot refuse.

    `unscored` is where ADR-010 §2 becomes observable, so at least one vector has to reach it —
    otherwise the most important rule in the engine is unexercised by the thing that exists to
    check the engine.
    """
    refusals = [
        conformance._vector_id(p)
        for p in conformance.engine_vector_paths()
        if conformance._read_json(p)["expected"]["swing"]["unscored"]
    ]
    assert refusals, "no committed vector produces an unscored checkpoint"

# --------------------------------------------------------------------------- M22 P1: the stages


def _engine_by_id() -> dict[str, Path]:
    return {conformance._vector_id(p): p for p in conformance.engine_vector_paths()}


def _stage_ids() -> list[str]:
    return [conformance._vector_id(p) for p in conformance.stage_vector_paths()]


def test_every_engine_vector_has_a_stage_vector() -> None:
    """Discovery, not a listing — an engine vector with no stages is a phase gate with a hole.

    The stages family exists so M22's phases are gated by a committed answer rather than by
    review (ADR-032 §2). A vector that is in the engine family and not in this one is a swing the
    port is judged on at the end and on nothing in between, which is the exact failure the family
    was added to prevent — and it would arrive silently, because `check` defers these to
    `cargo test` and would not miss them.
    """
    missing = sorted(set(_engine_by_id()) - {i.removeprefix("stages/") for i in _stage_ids()})
    # No command creates one from M32: `regenerate --stages-only` is refused (ADR-035 clause 3),
    # and `golf-core rerecord` re-records the stage vectors that exist rather than adding any.
    # A new vector is §M29's Rust vector builder's, stages included.
    assert not missing, (
        f"no stage vector for {missing} — from M32 a new vector and its stages come from the "
        f"Rust vector builder (§M29), not from `regenerate`"
    )


@pytest.mark.parametrize(
    "path", conformance.stage_vector_paths(), ids=conformance._vector_id
)
def test_each_stage_vector_names_an_input_that_exists(path: Path) -> None:
    """The input is held by reference, so the reference has to resolve.

    This family duplicates none of the 8.6 MB of committed inputs — it names the engine vector it
    was derived from and stops. That is only safe while the name resolves: a dangling
    `derived_from` is a file of expected answers with no question attached to it.
    """
    vector = conformance._read_json(path)
    assert vector["provenance"]["kind"] == "stages"
    derived = vector["provenance"].get("derived_from")
    assert derived in _engine_by_id(), (
        f"{path.name} says it derives from {derived!r}, which is not a committed engine vector"
    )


@pytest.mark.parametrize(
    "path", conformance.stage_vector_paths(), ids=conformance._vector_id
)
def test_each_stage_vector_is_at_the_frozen_version_or_ledgered_above_it(path: Path) -> None:
    """Same rule as the engine family, and it bites earlier.

    A stale *bundle* vector certifies a finished port against retracted answers. A stale *stage*
    vector does it four phases sooner, to a port that then builds everything after it on top —
    which is why `conformance.py check` reads these versions even though it defers running them.
    From M32 the rule is `ledger_covers`, as the engine family's is: at frozen Python's version,
    or above it with a `rerecords` entry for each version between. Equality with the current
    version is pinned against Rust's constant, in `crates/core/tests/stages.rs`.
    """
    vector = conformance._read_json(path)
    assert conformance.ledger_covers(vector, ANALYSIS_VERSION), (
        f"{path.name}: {conformance._staleness(vector)}"
    )


@pytest.mark.parametrize(
    "path", conformance.stage_vector_paths(), ids=conformance._vector_id
)
def test_each_stage_vector_is_what_this_build_produces(path: Path) -> None:
    """The committed intermediates, against this engine. Nothing else checks these bytes.

    `check` defers the family to `cargo test`, which is right — `crates/analysis` is the
    implementation under test. But that leaves the Python side unwatched, and the Python side is
    what *recorded* the file: a change to `smoothing.py` that moves a landmark without moving any
    final score would leave 21 committed files quietly lying to the port reading them. This is
    the only thing in the repo that would notice.

    Compared in `frozen_view` from M32, as the engine family is. M32's declaration reaches no
    stage value, only the stage vector's top-level `analysis_version`, so the view takes nothing
    out today. A later declaration that moves a stage value is what it is for.
    """
    vector = conformance._read_json(path)
    engine = conformance._read_json(_engine_by_id()[vector["provenance"]["derived_from"]])
    expected, actual = conformance.frozen_view(vector, conformance.run_stages(engine))
    diffs = conformance.compare_results(expected, actual)
    assert not diffs, "\n".join(
        ["frozen Python disagrees with the committed stage vector:"]
        + [f"  {d}" for d in diffs[:10]]
    )


@pytest.mark.parametrize(
    "path", conformance.stage_vector_paths(), ids=conformance._vector_id
)
def test_each_stage_vector_still_composes_onto_its_bundle_answer(path: Path) -> None:
    """The guard that makes these evidence rather than twenty-one self-consistent files.

    `run_stages` re-orchestrates the engine, and the order it calls things in is a second copy of
    `analyze_swing_bundle`'s. The composition check is what catches that copy drifting: every
    stage that reaches the artifact is composed forward and compared against the committed
    bundle. It runs at build time, and here too — because a build-time-only guard stops running
    the moment nobody regenerates, which on this family is most of the time.
    """
    sys.path.insert(0, str(REPO / "scripts"))
    from conformance_vectors import _verify_stages_compose

    vector = conformance._read_json(path)
    engine = conformance._read_json(_engine_by_id()[vector["provenance"]["derived_from"]])
    _verify_stages_compose(vector["stages"], engine)


def test_the_recorded_stages_are_exactly_the_ones_the_runner_names() -> None:
    """A stage added to `run_stages` without a regeneration is a key nothing committed holds."""
    for path in conformance.stage_vector_paths():
        got = set(conformance._read_json(path)["stages"])
        assert got == set(conformance.STAGE_NAMES), (
            f"{path.name} holds {sorted(got)}, runner names {sorted(conformance.STAGE_NAMES)}"
        )


def test_feedback_is_not_a_stage_and_that_is_deliberate() -> None:
    """ADR-032 §2 lists `feedback` as a ninth stage. It is not recorded, and this says why.

    `build_feedback` takes the *assembled* `SwingResult`, which no stage produces and this family
    does not hold — so a port cannot run it from a stage vector at all, and by the time it can,
    `expected.feedback` on the engine vector already gates it (M22 P6). Recording it would be a
    second copy of an answer committed a few hundred bytes away.

    Pinned rather than left to a comment because the *absence* is the decision, and an absence
    reads as an oversight to everyone who did not make it — including the session that regenerates
    this family after the next `ANALYSIS_VERSION` bump.
    """
    assert "feedback" not in conformance.STAGE_NAMES
    for path in conformance.engine_vector_paths():
        assert conformance._read_json(path)["expected"].get("feedback"), (
            f"{path.name} carries no feedback, so nothing gates `feedback/rules.py` at all"
        )


# ----------------------------------------------------------------------- M22 P3: the format table


def test_the_format_family_is_committed_and_covers_every_formatting_edge() -> None:
    """Thirteen vectors, and a port that reads the table can solve every CPython edge found so far.

    Discovery rather than a listing, the same choice the rest of this file makes — but the reason
    here is sharper: `check` *defers* this family to `cargo test`, so the Python suite is the only
    thing that would notice it going missing, and a `pyfmt` with no table under it is exactly the
    "gated by review" state ADR-032 §2 exists to forbid.

    **Five for the engine's four edges**, because ADR-032 §3 names three and M22 P5 found a
    fourth: `str(x)` on a float, which an f-string with no format spec reaches and which Rust's
    `{}` renders under different rules. `rounding` covers edge 1 in both its arities. (M22 P4
    found a fifth edge — Python's `max` returns the first maximum — which is not CPython
    *formatting* and is gated by the stage vectors rather than here.) **And five for the screen
    parser's** (M34 P2): `repr` on a `str`, float `//`, string case and whitespace, `sum`, and
    `difflib`. **And three for the many-shot layer's** (M36 P4): `:.Ng` at another precision,
    `str.lower()`, and pydantic's `datetime`.
    """
    by_id = {
        conformance._vector_id(path): conformance._read_json(path)
        for path in conformance.format_vector_paths()
    }
    assert set(by_id) == {
        "format/rounding",
        "format/fixed",
        "format/general",
        "format/repr",
        "format/ordering",
        "format/str_repr",
        "format/floor_div",
        "format/text_case",
        "format/sum",
        "format/difflib_ratio",
        "format/general_precision",
        "format/lower",
        "format/timestamp",
    }, f"the format family is {sorted(by_id)} — run `regenerate --format-only`"
    for name, vector in by_id.items():
        assert vector["provenance"]["kind"] == "format", name
        assert vector["cases"], f"{name} records no cases"


def test_every_format_vector_names_the_crate_that_runs_it() -> None:
    """`difflib_ratio` is `crates/screen`'s, `timestamp` is `crates/contracts`', and every other
    format vector is `pyfmt`'s.

    The family stopped being one crate's at M34 P2, so `check`'s line and `tests/format.rs`'s
    discovery both read `provenance.implemented_by`. The five engine tables predate the key and
    mean `pyfmt` by its absence; every table since names its crate, which this holds to, so a new
    table cannot be silently claimed by whichever reader defaults it.
    """
    engine = {"rounding", "fixed", "general", "repr", "ordering"}
    owners = {}
    for path in conformance.format_vector_paths():
        provenance = conformance._read_json(path)["provenance"]
        name = path.name.removesuffix(".json")
        if name in engine:
            assert "implemented_by" not in provenance, f"{name} predates the key"
        else:
            assert "implemented_by" in provenance, f"{name} names no implementing crate"
        owners[name] = provenance.get("implemented_by", "pyfmt")
    assert {name for name, crate in owners.items() if crate != "pyfmt"} == {
        "difflib_ratio",
        "timestamp",
    }
    assert owners["difflib_ratio"] == "screen"
    assert owners["timestamp"] == "contracts"


def test_the_parser_tables_hold_the_cases_that_separate_cpython_from_the_obvious_port() -> None:
    """`//` that is not `floor(a / b)`, a `sum` that is not a left fold, the C0 separators.

    The rounding table's pin, for the parser's edges: each of these tables is only worth its
    bytes for the cases where CPython and the call a port reaches for disagree, and a table that
    drifted towards cases they agree on would keep passing in Rust after it stopped gating
    anything.
    """
    tables = {
        path.name: conformance._read_json(path)["cases"]
        for path in conformance.format_vector_paths()
    }
    floor_div = [
        case
        for case in tables["floor_div.json"]
        if math.floor(float(case["a"]) / float(case["b"])) != float(case["expected"])
    ]
    assert floor_div, "no `//` case separates CPython from `floor(a / b)`"
    assert any(case["a"] == "21.0" and case["b"] == "4.2" for case in floor_div)

    def left_fold(values: list[float]) -> float:
        total = 0.0
        for value in values:
            total += value
        return total

    compensated = [
        case
        for case in tables["sum.json"]
        if repr(left_fold([float(v) for v in case["values"]])) != case["expected"]
    ]
    assert len(compensated) * 3 > len(tables["sum.json"]), "the `sum` table stopped gating"

    space_sets = [case for case in tables["text_case.json"] if case["op"] == "space_set"]
    assert len(space_sets) == 2
    for case in space_sets:
        assert {0x1C, 0x1D, 0x1E, 0x1F} <= set(case["expected"]), case["source"]



def test_the_many_shot_tables_hold_the_cases_that_separate_cpython_from_the_obvious_port() -> None:
    """`:.3g`'s moved threshold and its ties, `Final_Sigma`, and pydantic's re-spellings. [M36 P4]

    The parser tables' pin, for M36's three: each table earns its bytes on the cases where CPython
    or pydantic and the call a port reaches for disagree — `:g` at the wrong precision, a lowercase
    that ignores its neighbours, a timestamp carried as its lexeme or ordered as a string — so a
    table drifting away from them would keep passing in Rust after it stopped gating anything.
    """
    tables = {
        path.name: conformance._read_json(path) for path in conformance.format_vector_paths()
    }
    general = {
        (case["value"], case["precision"]): case["expected"]
        for case in tables["general_precision.json"]["cases"]
    }
    assert general[("999.5", 3)] == "1e+03", "the exponent form starts at 10**P"
    assert general[("12.25", 3)] == "12.2" and general[("12.75", 3)] == "12.8", "ties to even"
    assert general[("9.996", 3)] == "10", "a decade crossed by rounding"

    lower = tables["lower.json"]["cases"]
    by_text = {case["value"]: case["expected"] for case in lower if case["op"] == "lower"}
    assert by_text["ΑΣ"] == "ας" and by_text["ΑΣΑ"] == "ασα" and by_text["Σ"] == "σ"
    assert by_text["Α'Σ"] == "α'ς", "a case-ignorable character is skipped"
    whole = dict(next(case for case in lower if case["op"] == "lower_map")["expected"])
    assert whole[0x0130] == "i̇", "İ lowers to two code points"
    assert whole[0x212A] == "k", "the Kelvin sign lowers into ASCII"

    timestamp = tables["timestamp.json"]
    assert timestamp["provenance"]["pydantic_version"], "the table names the pydantic it records"
    rows = timestamp["cases"]
    assert {row["kind"] for row in rows} == {"written", "respelled", "refused"}
    spelled = {row["value"]: row for row in rows}
    assert spelled["2026-08-04T12:00:00+00:00"]["pydantic"] == "2026-08-04T12:00:00Z"
    assert spelled["2026-08-04T12:00:00.12Z"]["pydantic"] == "2026-08-04T12:00:00.120000Z"
    assert spelled["2026-08-04T12:00:00Z"]["isoformat"] == "2026-08-04T12:00:00+00:00"
    assert spelled["2026-08-20T23:30:00-05:00"]["date"] == "2026-08-20", "the date in its offset"
    assert any(row["epoch_us"] is not None and row["epoch_us"] < 0 for row in rows), "pre-epoch"
    instants: dict[int, set[str]] = {}
    for row in rows:
        if row["epoch_us"] is not None:
            instants.setdefault(row["epoch_us"], set()).add(row["pydantic"][19:])
    assert any(len(offsets) > 1 for offsets in instants.values()), "one instant, two offsets"

def test_the_format_family_does_not_age_on_the_engine_version() -> None:
    """It records CPython's rules, not this engine's answers, and the field names say which.

    Pinned because the *absence* of `analysis_version` is the decision. `cmd_check` keys its
    staleness test on that field, so a format vector carrying one would report as stale on every
    `ANALYSIS_VERSION` bump with nothing to regenerate about it — and the next session would
    dutifully rebuild four files whose contents cannot have changed. What it ages on is CPython.
    """
    for path in conformance.format_vector_paths():
        vector = conformance._read_json(path)
        assert "analysis_version" not in vector, f"{path.name} claims an engine version"
        assert vector["python_version"].startswith("3."), path.name


def test_the_format_table_regenerates_byte_identically() -> None:
    """The one property that makes a seeded sweep safe to commit.

    Half the cases come from `random.Random(20260923)`, which is only acceptable while a rebuild on
    another machine produces the same file — otherwise this family shows up as a diff in every
    session that runs a full `regenerate`, and a diff nobody can explain is a diff nobody reads.
    Checked against what is on disk rather than against a second call, because agreeing with itself
    is not the claim.
    """
    sys.path.insert(0, str(Path(conformance.__file__).resolve().parent))
    from conformance_vectors import build_format

    for path, payload in build_format():
        assert path.exists(), f"{path} is not committed"
        assert payload == conformance._read_json(path), (
            f"{path.name} differs from a fresh build — run `regenerate --format-only`"
        )


def test_the_rounding_table_holds_the_exact_ties_and_not_only_a_sweep() -> None:
    """The cases that separate half-to-even from half-away-from-zero are the only ones that matter.

    A tie at `n` decimal places is an odd multiple of `2**-(n+1)` and nothing else is one, so they
    are enumerable — and a uniform sweep hits none of them. This asserts the table actually carries
    them, because a rounding gate made only of random values would pass a port with the wrong rule.
    """
    cases = conformance._read_json(
        next(p for p in conformance.format_vector_paths() if p.name == "rounding.json")
    )["cases"]
    for ndigits in (None, 1, 2, 4):
        step = 2.0 ** -((ndigits or 0) + 1)
        present = {
            float(case["value"])
            for case in cases
            if case["ndigits"] == ndigits and float(case["value"]) == float(case["value"])
        }
        ties = [step * (2 * i + 1) for i in range(8)]
        missing = [tie for tie in ties if tie not in present]
        assert not missing, f"ndigits={ndigits} is missing exact ties {missing}"


def test_every_ordering_case_is_a_tie() -> None:
    """A sort with distinct keys agrees under any algorithm, so a case without a tie gates nothing.

    This is the edge that produces a correct number under the wrong *name*, and it is only reachable
    when two keys collide. A case list that drifted towards distinct values would still pass in Rust
    and would have stopped testing anything, which is the failure mode worth a pin of its own.
    """
    cases = conformance._read_json(
        next(p for p in conformance.format_vector_paths() if p.name == "ordering.json")
    )["cases"]
    for case in cases:
        if case["key"] == "registry":
            ranks = [
                case["registry"].index(n) if n in case["registry"] else len(case["registry"])
                for n in case["names"]
            ]
            assert len(set(ranks)) < len(ranks), f"no collision in {case['names']}"
        else:
            keys = [
                -abs(float(v)) if case["key"] == "neg_abs" else -float(v)
                for _, v in case["entries"]
            ]
            assert len(set(keys)) < len(keys), f"no tie in {case['entries']}"


# ----------------------------------------------------------------------- M34 P4: the screen family

#: The sub-families frozen Python recorded through `regenerate --screen-once`. M34 P8 added
#: `hand/`, whose oracle is a person. P10 re-recorded three of these four through `golf-core
#: rerecord`, which keeps `provenance.oracle` as it found it. `units/` holds case tables, not
#: documents, so the verb never reads it, and it stays frozen Python's at version 0.
_PYTHON_RECORDED_SCREEN = ("corpus", "reference", "synthetic", "units")


def _screen_by_family() -> dict[str, list[tuple[Path, dict[str, Any]]]]:
    by_family: dict[str, list[tuple[Path, dict[str, Any]]]] = {}
    for path in conformance.screen_vector_paths():
        family = path.relative_to(conformance.VECTORS / "screen").parts[0]
        by_family.setdefault(family, []).append((path, conformance._read_json(path)))
    return by_family


def _copy_screen_family(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """The committed screen family, copied under a temporary `spec/vectors/`, with `conformance`
    pointed at it, so a pin can add to the family or try to write over it without touching `spec/`.
    """
    import shutil

    vectors = tmp_path / "spec" / "vectors"
    shutil.copytree(conformance.VECTORS / "screen", vectors / "screen")
    monkeypatch.setattr(conformance, "VECTORS", vectors)
    return vectors


def _snapshot(directory: Path) -> dict[str, bytes]:
    files = (p for p in directory.rglob("*") if p.is_file())
    return {p.relative_to(directory).as_posix(): p.read_bytes() for p in files}


def test_the_screen_family_is_committed_in_the_four_sub_families_python_recorded() -> None:
    """Each sub-family covers what the others cannot, so an empty one is a gate that went missing.

    `check` defers the whole family to `cargo test`, which makes this suite the only Python-side
    thing that notices it shrinking: the corpus is the real OCR, the reference photos the other
    layout, the synthetic screens the paths the parser tests take, and the units the edges no
    screen reaches.
    """
    by_family = _screen_by_family()
    for family in _PYTHON_RECORDED_SCREEN:
        assert by_family.get(family), f"spec/vectors/screen/{family}/ is empty"
        for path, vector in by_family[family]:
            assert vector["provenance"]["kind"] == "screen", path.name
            assert vector["provenance"]["oracle"] == "python", path.name
            assert vector["provenance"]["python_version"].startswith("3."), path.name


def test_the_screen_family_ages_on_the_parser_version_and_never_on_the_engines() -> None:
    """`screen_parser_version`, top-level where every family keeps its version, and no other.

    The format family's pin, for the screen: `check` keys staleness on `analysis_version`, so a
    screen vector carrying one would go stale on every engine bump with nothing to re-record about
    it. What it ages on is `SCREEN_PARSER_VERSION`, Rust's constant, and frozen Python's parse is
    entry 0 of its ledger.
    """
    for family, vectors in _screen_by_family().items():
        for path, vector in vectors:
            name = f"{family}/{path.name}"
            assert "analysis_version" not in vector, f"{name} claims an engine version"
            assert isinstance(vector.get("screen_parser_version"), int), name
            assert "screen_parser_version" not in vector["provenance"], f"{name}: version twice"
            if family in _PYTHON_RECORDED_SCREEN and not vector["provenance"].get("rerecords"):
                assert vector["screen_parser_version"] == 0, name


def test_a_screen_document_carries_what_a_port_reads() -> None:
    """Call 1's shape: boxes and notes in, the parse before validation and the record after it out.

    `expected.parsed` and `expected.shot` are split so `crates/screen` can be gated on its parser
    (P5) before it has a validator (P6), and `shot` is `None` exactly where `import_screen` returns
    `failed` — which the synthetic family has to reach at least once, or nothing gates that `None`.
    Supersets rather than equalities on `expected`, because P10's re-record adds keys there.
    """
    given_keys = {
        "device", "boxes", "notes", "shot_id", "session_id", "timestamp", "image_sha256",
        "image_path", "min_confidence",
    }
    box_keys = {"text", "x", "y", "width", "height", "confidence"}
    failed = 0
    for family, vectors in _screen_by_family().items():
        if family == "units":
            continue
        for path, vector in vectors:
            name = f"{family}/{path.name}"
            assert set(vector["input"]) == given_keys, name
            assert all(set(box) == box_keys for box in vector["input"]["boxes"]), name
            assert {"label_ratio", "parsed", "shot"} <= set(vector["expected"]), name
            assert {"values", "raw_fields", "confidence", "warnings"} <= set(
                vector["expected"]["parsed"]
            ), name
            failed += vector["expected"]["shot"] is None
    assert failed, "no screen vector records a failed read, so nothing gates `shot: None`"


def test_a_photo_vector_names_its_photo_portably_and_once() -> None:
    """Repo-relative `/` paths, one vector per distinct photo, and the OCR that read it.

    The store holds absolute Windows paths, and a vector must read the same on every machine.
    Deduplicated on the photo's sha256 because one photo can be the shot screen of several
    bundles, and a vector per bundle would be several files recording one parse.
    """
    by_family = _screen_by_family()
    for family in ("corpus", "reference"):
        digests = []
        for path, vector in by_family[family]:
            given = vector["input"]
            image_path = given["image_path"]
            assert "\\" not in image_path and not re.match(r"^([A-Za-z]:|/)", image_path), path.name
            assert image_path.startswith("data/"), path.name
            assert re.fullmatch(r"[0-9a-f]{64}", given["image_sha256"]), path.name
            assert vector["provenance"]["paddleocr_version"], path.name
            digests.append(given["image_sha256"])
            if family == "corpus":
                assert given["shot_id"] == path.name.removesuffix(".json"), path.name
        assert len(digests) == len(set(digests)), f"two {family} vectors record one photo"


def test_the_synthetic_screens_gate_cpythons_compensated_sum(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """M34 P2 found OCR confidences are float32, which sum exactly either way, so no corpus photo
    can tell CPython's compensated `sum` from a left fold — the synthetic screens must.

    Run on the vectors' *inputs* through frozen Python with `sum` swapped for a left fold, not
    against `expected`, so the pin still measures the inputs after P10 re-records the answers.
    """
    from golf_coach.launch_monitor.screen import parser
    from golf_coach.launch_monitor.screen.profiles import load_profile
    from golf_coach.launch_monitor.screen.recognizer import TextBox

    def left_fold(values: Any, start: float = 0) -> Any:
        total = start
        for value in values:
            total += value
        return total

    profile = load_profile("hd_golf")
    inputs = [
        [TextBox(**box) for box in vector["input"]["boxes"]]
        for _, vector in _screen_by_family()["synthetic"]
    ]
    compensated = [parser.parse_screen(boxes, profile).confidence for boxes in inputs]
    # A module global shadows the builtin for every `sum(...)` in `parser`, `_score`'s included.
    monkeypatch.setattr(parser, "sum", left_fold, raising=False)
    folded = [parser.parse_screen(boxes, profile).confidence for boxes in inputs]
    assert any(a != b for a, b in zip(compensated, folded, strict=True)), (
        "no synthetic screen's confidence depends on how `sum` adds — `pyfmt::sum` is ungated"
    )


def test_check_defers_the_screen_family_and_judges_no_staleness(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """Named and deferred, never run, and never STALE — even at a version frozen Python lacks.

    Frozen Python has no `SCREEN_PARSER_VERSION`, so it has nothing to call a screen vector stale
    against, and `golf-core rerecord` is the only thing that moves the version. A vector far above
    anything recorded is here to prove `check` does not try.
    """
    vectors = _copy_screen_family(tmp_path, monkeypatch)
    ahead = next(vectors.rglob("*.json"))
    document = json.loads(ahead.read_text(encoding="utf-8"))
    document["screen_parser_version"] = 99
    (ahead.parent / "ahead.json").write_text(json.dumps(document), encoding="utf-8")

    def never(vector: dict[str, Any]) -> dict[str, Any]:
        raise AssertionError(f"`check` ran screen vector {vector.get('id')} through the engine")

    monkeypatch.setattr(conformance, "run_vector", never)
    assert conformance.main(["check"]) == 0
    out = capsys.readouterr().out
    count = len(conformance.screen_vector_paths())
    assert f"{count} screen vectors are `crates/screen`'s" in out
    assert "STALE" not in out


@pytest.mark.parametrize("what", ["a committed vector", "any file at all"])
def test_screen_once_refuses_while_the_family_has_anything_in_it(
    what: str,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Once means once: a second run from frozen Python would write over what Rust re-recorded.

    Refused on any file under `screen/`, not only on one that reads as a screen vector, because
    the writer would go over either. Refused before the recorder is imported, so a refusal cannot
    reach PaddleOCR or `data/`, and it writes nothing.
    """
    if what == "a committed vector":
        vectors = _copy_screen_family(tmp_path, monkeypatch)
    else:
        vectors = tmp_path / "spec" / "vectors"
        (vectors / "screen").mkdir(parents=True)
        (vectors / "screen" / "README").write_text("not a vector", encoding="utf-8")
        monkeypatch.setattr(conformance, "VECTORS", vectors)
    before = _snapshot(vectors)
    calls = _refuse_to_build(monkeypatch)

    assert conformance.main(["regenerate", "--screen-once"]) == 2
    assert not calls, f"a refused `--screen-once` still reached {calls}"
    assert _snapshot(vectors) == before
    assert "golf-core rerecord" in capsys.readouterr().err


def test_screen_once_records_an_empty_family_and_then_refuses(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """The one run writes what `build_screen` returned, and the next run is refused by it."""
    vectors = tmp_path / "spec" / "vectors"
    vectors.mkdir(parents=True)
    monkeypatch.setattr(conformance, "VECTORS", vectors)
    recorded = vectors / "screen" / "synthetic" / "only.json"
    builds: list[str] = []

    def build_screen() -> list[tuple[Path, dict[str, Any]]]:
        builds.append("build_screen")
        return [(recorded, {"id": "screen/synthetic/only", "provenance": {"kind": "screen"}})]

    fake = types.ModuleType("conformance_vectors")
    fake.build_screen = build_screen  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "conformance_vectors", fake)

    assert conformance.main(["regenerate", "--screen-once"]) == 0
    assert recorded.exists()
    assert conformance.main(["regenerate", "--screen-once"]) == 2
    assert builds == ["build_screen"], "the second run reached the recorder"


# ------------------------------------------------------------- M36 P1: the storage corpus cases

#: Where the real corpus is read from. Spelled here rather than imported, so the skip below is
#: decided at collection without loading the recorder.
_SESSIONS = REPO / "data" / "processed" / "sessions"


@functools.cache
def _storage_corpus_built() -> tuple[tuple[Path, dict[str, Any]], ...]:
    """The synthetic corpus cases, built once per run: each pin below reads the same build."""
    from conformance_vectors import build_storage_corpus

    return tuple(build_storage_corpus(real=False))


def test_the_storage_corpus_cases_build_in_memory_and_write_nothing() -> None:
    """P1's dry run: every case builds, carries an answer, and has a name of its own.

    In memory only — `regenerate --storage-once` (P2) is the one writer — so whatever is under
    `spec/vectors/storage/` is byte-identical afterwards, nothing included.
    """
    storage = conformance.VECTORS / "storage"
    before = _snapshot(storage) if storage.exists() else {}
    built = _storage_corpus_built()
    assert (_snapshot(storage) if storage.exists() else {}) == before

    ids = [vector["id"] for _, vector in built]
    assert len(ids) == len(set(ids)), "two storage corpus cases share an id"
    assert len({path for path, _ in built}) == len(built), "two cases would write one file"
    for path, vector in built:
        name = vector["id"]
        assert path.is_relative_to(conformance.VECTORS / "storage" / "corpus"), name
        assert name == f"storage/corpus/{path.name.removesuffix('.json')}", name
        assert vector["career_version"] == 0, name
        assert "analysis_version" not in vector, f"{name} would age on the engine version"
        assert vector["provenance"]["kind"] == "storage", name
        assert vector["provenance"]["oracle"] == "python", name
        assert set(vector["input"]) == {"player_id", "versions", "files", "narrowings"}, name
        expected = vector["expected"]
        assert set(expected) == {"corpus", "properties", "narrowed"}, name
        assert set(expected["narrowed"]) == set(vector["input"]["narrowings"]), name


def test_the_storage_corpus_cases_reach_every_exclusion_and_both_mishit_halves() -> None:
    """The rows a port is most likely to answer differently, each reached at least once.

    Every `ExclusionReason`, the automatic flag and both verdicts, a narrowing, and a root that
    does not exist, because `check` defers this family to `cargo test` and nothing else on the
    Python side would notice a case list that quietly lost one.
    """
    from golf_coach.contracts.career import ExclusionReason
    from golf_coach.contracts.mishit import MishitVerdict

    reasons: set[str] = set()
    verdicts: set[str] = set()
    auto = narrowed = 0
    missing_root = False
    for _, vector in _storage_corpus_built():
        corpus = vector["expected"]["corpus"]
        reasons |= {entry["reason"] for entry in corpus["excluded"]}
        verdicts |= {swing["manual_mishit"] for swing in corpus["swings"]} - {None}
        auto += sum(swing["auto_mishit"] for swing in corpus["swings"])
        narrowed += len(vector["expected"]["narrowed"])
        missing_root |= vector["input"]["files"] is None
    assert reasons == {reason.value for reason in ExclusionReason}
    assert verdicts == {verdict.value for verdict in MishitVerdict}
    assert auto, "no case flags a mishit automatically"
    assert narrowed, "no case narrows its corpus"
    assert missing_root, "no case reads a sessions root that does not exist"


def test_frozen_python_answers_a_corpus_case_only_under_its_own_engine() -> None:
    """`input.versions` is the parameter Rust's `read_corpus` takes (the plan's call 7). Frozen
    Python reads `ANALYSIS_VERSION` for itself, so any other pair is a question it cannot answer,
    and the recorder says so rather than recording an answer to a different question."""
    from conformance_vectors import _run_corpus

    given = {
        "player_id": "aaron",
        "versions": {"installed": ANALYSIS_VERSION + 1, "comparable_from": ANALYSIS_VERSION - 2},
        "files": {},
        "narrowings": {},
    }
    with pytest.raises(ValueError, match="answers only under"):
        _run_corpus(given)


@pytest.mark.skipif(
    not _SESSIONS.is_dir(),
    reason="no data/processed/sessions/: the real storage corpus case is read from the captures",
)
def test_the_real_corpus_case_reads_exactly_as_data_does() -> None:
    """The slimmed tree is a claim about what `read_corpus` reads (the plan's finding 11), and
    `_storage_corpus_real` raises unless reading it equals reading `data/` itself. Gzipped, since
    P1 measured it at 480 KB plain."""
    from conformance_vectors import _REAL_SUFFIX, _storage_corpus_real

    vector = _storage_corpus_real()
    assert _REAL_SUFFIX == ".json.gz"
    assert vector["id"] == "storage/corpus/real"
    assert vector["expected"]["corpus"]["swings"], "the real corpus pooled no swing"
    assert vector["input"]["narrowings"], "the real corpus holds no tagged club"
    for rel in vector["input"]["files"]:
        assert not rel.startswith("."), f"{rel}: a dotted directory `read_corpus` never enters"
        assert rel.endswith(("/", ".json")), f"{rel}: not a file `read_corpus` reads"


# ------------------------------------------------------------ M36 P2: the store operations, on disk

#: The sub-families frozen Python recorded through `regenerate --storage-once`. M36 P14 adds
#: `hand/`, whose oracle is a person, and re-records these through `golf-core rerecord`, which
#: keeps `provenance.oracle` as it found it.
_PYTHON_RECORDED_STORAGE = ("corpus", "bundle", "stores")


@functools.cache
def _storage_ops_built() -> tuple[tuple[Path, dict[str, Any]], ...]:
    """The operation cases, built once per run: each pin below reads the same build."""
    from conformance_vectors import build_storage_ops

    return tuple(build_storage_ops())


def _copy_storage_family(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """The committed storage family under a temporary `spec/vectors/`, `_copy_screen_family`'s
    way, so a pin can add to it or try to write over it without touching `spec/`."""
    import shutil

    vectors = tmp_path / "spec" / "vectors"
    shutil.copytree(conformance.VECTORS / "storage", vectors / "storage")
    monkeypatch.setattr(conformance, "VECTORS", vectors)
    return vectors


def test_the_storage_ops_cases_build_in_memory_and_write_nothing() -> None:
    """The plan's call 1: a tree and calls in, each call's answer and the tree after out.

    One answer per call, each exactly one of `returned` and `raised`, and every call carrying the
    `now` it was run under (`null` for one that may not read a clock). Built in memory, so
    `spec/vectors/storage/` is byte-identical afterwards.
    """
    storage = conformance.VECTORS / "storage"
    before = _snapshot(storage) if storage.exists() else {}
    built = _storage_ops_built()
    assert (_snapshot(storage) if storage.exists() else {}) == before

    ids = [vector["id"] for _, vector in built]
    assert len(ids) == len(set(ids)), "two storage op cases share an id"
    for path, vector in built:
        name = vector["id"]
        family = path.parent.name
        assert family in {"bundle", "stores"}, name
        assert name == f"storage/{family}/{path.name.removesuffix('.json')}", name
        assert vector["career_version"] == 0, name
        assert "analysis_version" not in vector, f"{name} would age on the engine version"
        assert "analysis_version" not in vector["provenance"], f"{name}: no op reads an engine"
        assert vector["provenance"]["recorded_by"].endswith("::_run_ops"), name
        given, expected = vector["input"], vector["expected"]
        assert set(given) == {"files", "ops"}, name
        assert set(expected) == {"results", "files"}, name
        assert len(expected["results"]) == len(given["ops"]), name
        for op, result in zip(given["ops"], expected["results"], strict=True):
            assert set(op) == {"op", "args", "now"}, f"{name}: {op}"
            assert len(result) == 1 and set(result) <= {"returned", "raised"}, f"{name}: {result}"


def test_the_storage_ops_cases_reach_every_op_and_every_kind_of_answer() -> None:
    """Every op the recorder knows, and the answers a port is likeliest to get wrong.

    `check` defers this family to `cargo test`, so nothing else on the Python side would notice a
    case list that quietly stopped reaching a raise, a dedupe, an explicit target or a verdict.
    """
    from conformance_vectors import _op_table

    reached: set[str] = set()
    verdicts: set[str | None] = set()
    deduped = targeted = with_message = without_message = absent_roots = 0
    for _, vector in _storage_ops_built():
        absent_roots += vector["input"]["files"] is None
        for op, result in zip(vector["input"]["ops"], vector["expected"]["results"], strict=True):
            reached.add(op["op"])
            if op["op"] == "bundle.set_mishit":
                verdicts.add(op["args"]["verdict"])
            if op["op"] == "bundle.assign_from_path" and "returned" in result:
                deduped += result["returned"]["deduped"]
                targeted += op["args"]["swing_id"] is not None
            if "raised" in result:
                with_message += result["raised"]["message"] is not None
                without_message += result["raised"]["message"] is None
    assert reached == set(_op_table()), f"never reached: {set(_op_table()) - reached}"
    assert verdicts == {"confirmed", "cleared", None}
    assert deduped and targeted, "no upload is deduped, or none names its swing"
    assert with_message and without_message, "a raise of each kind (ours, pydantic's) is missing"
    assert absent_roots, "no case runs on a root that does not exist"


def test_an_op_recorded_without_a_clock_may_not_read_one() -> None:
    """`now: null` is a claim the recorder checks: a call that stamps, given no instant, stops
    the build rather than recording whatever this machine's clock said (the plan's call 4)."""
    from conformance_vectors import _run_ops

    given = {
        "files": {},
        "ops": [
            {
                "op": "golfer.get_or_create",
                "args": {"name": "Aaron", "handedness": "right"},
                "now": None,
            }
        ],
    }
    with pytest.raises(AssertionError, match="read the clock"):
        _run_ops(given)


def test_the_storage_family_is_committed_in_the_sub_families_python_recorded() -> None:
    """Recorded once, by `regenerate --storage-once`, at `career_version` 0 unless a Rust
    re-record has ledgered it since, and never at an engine version: `read_corpus` takes the
    engine's as a parameter (the plan's decision 8)."""
    by_family: dict[str, list[Path]] = {}
    for path in conformance.storage_vector_paths():
        family = path.relative_to(conformance.VECTORS / "storage").parts[0]
        by_family.setdefault(family, []).append(path)
    for family in _PYTHON_RECORDED_STORAGE:
        assert by_family.get(family), f"spec/vectors/storage/{family}/ is empty"
        for path in by_family[family]:
            vector = conformance._read_json(path)
            name = vector["id"]
            assert name == conformance._vector_id(path), f"{path.name} names itself {name}"
            assert vector["provenance"]["kind"] == "storage", name
            assert vector["provenance"]["oracle"] == "python", name
            assert vector["provenance"]["python_version"].startswith("3."), name
            assert "analysis_version" not in vector, f"{name} claims an engine version"
            assert isinstance(vector.get("career_version"), int), name
            # `conformance.py list` prints every note, and a Windows console encodes as cp1252:
            # M36 P2's first recording carried a U+0130 that made `list` raise there.
            vector["provenance"]["note"].encode("cp1252")
            if not vector["provenance"].get("rerecords"):
                assert vector["career_version"] == 0, name
    assert (conformance.VECTORS / "storage" / "corpus" / "real.json.gz").is_file()


def test_check_defers_the_storage_family_and_judges_no_staleness(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """Named and deferred, never run, and never STALE — even at a version frozen Python lacks,
    `test_check_defers_the_screen_family_and_judges_no_staleness`'s pin for `CAREER_VERSION`."""
    vectors = _copy_storage_family(tmp_path, monkeypatch)
    ahead = next(vectors.rglob("*.json"))
    document = json.loads(ahead.read_text(encoding="utf-8"))
    document["career_version"] = 99
    (ahead.parent / "ahead.json").write_text(json.dumps(document), encoding="utf-8")

    def never(vector: dict[str, Any]) -> dict[str, Any]:
        raise AssertionError(f"`check` ran storage vector {vector.get('id')} through the engine")

    monkeypatch.setattr(conformance, "run_vector", never)
    assert conformance.main(["check"]) == 0
    out = capsys.readouterr().out
    count = len(conformance.storage_vector_paths())
    assert f"{count} storage vectors are `crates/storage`'s" in out
    assert "STALE" not in out


def test_list_shows_a_storage_vector_at_its_career_version(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """`list` prints the first version key a vector carries; without `career_version` in that
    tuple a storage vector would print `vNone`. Each row shows the vector's own version, read off
    the file: 0 as frozen Python recorded them, and whatever `golf-core rerecord` has moved them to
    since (M36 P14's `career-v1`, and the hand-worked cases, which start there)."""
    _copy_storage_family(tmp_path, monkeypatch)
    versions = {
        conformance._vector_id(path): conformance._read_json(path)["career_version"]
        for path in conformance.storage_vector_paths()
    }
    assert conformance.main(["list"]) == 0
    rows = [line for line in capsys.readouterr().out.splitlines() if line.startswith("storage/")]
    assert rows, "`list` showed no storage vector"
    assert len(rows) == len(versions), rows[:3]
    for row in rows:
        version = versions[row.split()[0]]
        assert isinstance(version, int), row
        assert re.search(rf"\s+v{version}\s+storage\s", row), row


@pytest.mark.parametrize("what", ["a committed vector", "any file at all"])
def test_storage_once_refuses_while_the_family_has_anything_in_it(
    what: str,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Once means once, `--screen-once`'s rule: refused on any file under `storage/`, before the
    recorder is imported, so a refusal cannot reach `data/`, and it writes nothing."""
    if what == "a committed vector":
        vectors = _copy_storage_family(tmp_path, monkeypatch)
    else:
        vectors = tmp_path / "spec" / "vectors"
        (vectors / "storage").mkdir(parents=True)
        (vectors / "storage" / "README").write_text("not a vector", encoding="utf-8")
        monkeypatch.setattr(conformance, "VECTORS", vectors)
    before = _snapshot(vectors)
    calls = _refuse_to_build(monkeypatch)

    assert conformance.main(["regenerate", "--storage-once"]) == 2
    assert not calls, f"a refused `--storage-once` still reached {calls}"
    assert _snapshot(vectors) == before
    message = capsys.readouterr().err
    assert "golf-core rerecord" in message
    assert "career-v<N>.json" in message


def test_storage_once_records_an_empty_family_and_then_refuses(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """The one run writes what `build_storage` returned, and the next run is refused by it."""
    vectors = tmp_path / "spec" / "vectors"
    vectors.mkdir(parents=True)
    monkeypatch.setattr(conformance, "VECTORS", vectors)
    recorded = vectors / "storage" / "stores" / "only.json"
    builds: list[str] = []

    def build_storage() -> list[tuple[Path, dict[str, Any]]]:
        builds.append("build_storage")
        return [(recorded, {"id": "storage/stores/only", "provenance": {"kind": "storage"}})]

    fake = types.ModuleType("conformance_vectors")
    fake.build_storage = build_storage  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "conformance_vectors", fake)

    assert conformance.main(["regenerate", "--storage-once"]) == 0
    assert recorded.exists()
    assert conformance.main(["regenerate", "--storage-once"]) == 2
    assert builds == ["build_storage"], "the second run reached the recorder"


# ------------------------------------------------ M36 P3: the career aggregates and the reports

#: Where the real career case reads the golfer's record and bag. Spelled here for the skip, as
#: `_SESSIONS` is.
_GOLFERS = REPO / "data" / "processed" / "golfers"

#: The reports every career case records plain and `--verbose`, by script.
_CAREER_REPORTS = ("career_corpus", "career_baseline", "career_dispersion", "club_profile")

#: A sentence each branch of the five reports prints, by the report key it appears under
#: (`club_profile_*` is every per-club key). Reached by at least one case, or a renderer branch
#: has no recorded text to be gated by.
_CAREER_BRANCHES: dict[str, tuple[str, ...]] = {
    "career_corpus": (
        "Collapsed",
        "disagree on the shot photo",
        "analyzed by an older engine",
        "carrying no measurement at all",
        "naming no club",
        "Unrecognised measurement sources",
        "Contributing no sample",
        "belong to someone else",
        "no distinct swing carries a measurement yet",
    ),
    "career_corpus_verbose": ("not_analyzed — no analysis.json",),
    "career_baseline": (
        "Nothing is sayable yet",
        "No measurement on any swing yet",
        "trend    per session:",
        "inside tour range",
        "above tour range",
        "below tour range",
        "cannot tell",
        "at least 90th pct",
        "at least 10th pct",
        "past the edge",
        "the spread is not placed against the tour spread",
        "no tour population exists",
        "it is a model output",
        "no reference distribution is stored",
        "has a stored distribution and may not be placed in it",
    ),
    "career_baseline_verbose": ("sessions so far:",),
    "career_dispersion": (
        "No metric can be asked yet",
        "No measurement on any swing yet",
        "    pattern   biased and scattered",
        "    caveat    Pooled spread",
        "    waiting   ",
        "    blocked   ",
        "within-session",
    ),
    "career_dispersion_verbose": ("    tolerance ",),
    "club_profile": (
        "No club has been hit or declared",
        "No swings on record either",
        "on record name no club. That is real history",
        "In the bag, nothing hit with it yet",
        "These swings carry no measurement",
        "capped at the shot count",
        "No bag entry declared",
        "make and model not recorded",
        "loft not recorded",
        "nothing above counts them",
        "    finding   bias ",
    ),
    "club_profile_verbose": ("sessions so far:",),
    "club_profile_*": ("never hit and not in the bag",),
    "flag_mishit_list": ("(no tagged shots yet)", "   clean", "auto and unconfirmed"),
}


@functools.cache
def _career_built() -> tuple[tuple[Path, dict[str, Any]], ...]:
    """The synthetic career cases, built once per run: each pin below reads the same build."""
    from conformance_vectors import build_career

    return tuple(build_career(real=False))


def _copy_career_family(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """The committed career family under a temporary `spec/vectors/`, `_copy_storage_family`'s
    way, so a pin can add to it or try to write over it without touching `spec/`."""
    import shutil

    vectors = tmp_path / "spec" / "vectors"
    shutil.copytree(conformance.VECTORS / "career", vectors / "career")
    monkeypatch.setattr(conformance, "VECTORS", vectors)
    return vectors


def test_the_career_cases_build_in_memory_and_write_nothing() -> None:
    """The plan's call 2, as P3 made it concrete: a corpus, a bag and a name in; the four
    aggregates, what they derive, and every report out. Built in memory, so whatever is under
    `spec/vectors/career/` is byte-identical afterwards."""
    career = conformance.VECTORS / "career"
    before = _snapshot(career) if career.exists() else {}
    built = _career_built()
    assert (_snapshot(career) if career.exists() else {}) == before

    ids = [vector["id"] for _, vector in built]
    assert len(ids) == len(set(ids)), "two career cases share an id"
    for path, vector in built:
        name = vector["id"]
        assert path.is_relative_to(conformance.VECTORS / "career" / "synthetic"), name
        assert name == f"career/synthetic/{path.name.removesuffix('.json')}", name
        assert vector["career_version"] == 0, name
        assert "analysis_version" not in vector, f"{name} would age on the engine version"
        assert vector["provenance"]["kind"] == "career", name
        assert vector["provenance"]["oracle"] == "python", name
        assert vector["provenance"]["recorded_by"].endswith("::_run_career"), name
        given, expected = vector["input"], vector["expected"]
        assert set(given) == {"corpus", "bag", "display_name", "versions", "clubs"}, name
        assert set(expected) == {
            "baseline",
            "dispersion",
            "standing",
            "bag_profile",
            "properties",
            "reports",
        }, name
        keys = {f"{script}{tail}" for script in _CAREER_REPORTS for tail in ("", "_verbose")}
        keys |= {f"club_profile_{club}" for club in given["clubs"]}
        assert set(expected["reports"]) == keys | {"flag_mishit_list"}, name
        # Every club the profile holds, in its order, then one it does not.
        profiled = [one["club"] for one in expected["bag_profile"]["clubs"]]
        assert given["clubs"][:-1] == profiled, name
        assert given["clubs"][-1] not in profiled, name


def test_the_career_cases_cross_every_floor_and_reach_every_reading() -> None:
    """The rows a port is likeliest to answer differently, each reached at least once.

    Every floor of the M36 plan's finding 4 shut at n - 1 and open at n, and the TREND sessions
    gate shut at 2 and open at 3; every `Standing`, both sides of the band and every
    `DispersionPattern`, the scatter-only reading and the drift caveat on both sides of its factor;
    every caveat form a club profile writes; an interval each side of the critical-value tables'
    edge. `check` defers this family to `cargo test`, so nothing else on the Python side would
    notice a case list that quietly lost one.
    """
    from golf_coach.contracts.baseline import BaselineClaim, minimum_n, minimum_sessions
    from golf_coach.contracts.comparison import Standing
    from golf_coach.contracts.dispersion import SCATTER_ONLY_READING, DispersionPattern

    floored = ("head_sway_norm", "tempo_ratio", "hip_shift_at_top_norm")
    crossed: set[tuple[str, str, int]] = set()
    sessions_gate: set[tuple[int, bool]] = set()
    standings: set[str] = set()
    sides: set[bool] = set()
    patterns: set[str] = set()
    readings: set[str] = set()
    drift: set[bool] = set()
    caveats: list[str] = []
    intervals: set[int] = set()
    for _, vector in _career_built():
        expected = vector["expected"]
        for name, metric in expected["baseline"]["metrics"].items():
            if metric["sd_ci"] is not None:
                intervals.add(metric["n"])
            for claim in BaselineClaim:
                ready = claim.value in metric["ready"]
                need = minimum_n(name, claim)
                enough_sessions = metric["n_sessions"] >= minimum_sessions(claim)
                if enough_sessions and metric["n"] in (need - 1, need):
                    assert ready is (metric["n"] == need), (vector["id"], name, claim)
                    crossed.add((name, claim.value, metric["n"] - need))
                if claim is BaselineClaim.TREND and metric["n"] >= need:
                    sessions_gate.add((metric["n_sessions"], ready))
        for metric in expected["standing"]["metrics"].values():
            standings.add(metric["standing"])
            if metric["outside_by"] is not None:
                sides.add(metric["outside_by"] > 0)
        for metric in expected["dispersion"]["metrics"].values():
            if metric["pattern"] is not None:
                patterns.add(metric["pattern"])
            elif metric["points_at"] is not None:
                readings.add(metric["points_at"])
            if metric["within_session_sd"] is not None:
                drift.add(bool(metric["caveats"]))
        caveats += [c for club in expected["bag_profile"]["clubs"] for c in club["caveats"]]

    every = {(name, claim.value) for name in floored for claim in BaselineClaim}
    assert crossed >= {(*row, offset) for row in every for offset in (-1, 0)}, "a floor uncrossed"
    assert {(2, False), (3, True)} <= sessions_gate, sessions_gate
    # `analysis/stats.py`'s tables end at df 30 and its expansions start at 31.
    assert 31 in intervals and max(intervals) > 32, "no interval past the critical-value tables"
    assert standings == {standing.value for standing in Standing}
    assert sides == {True, False}, "an OUTSIDE standing on only one side of the band"
    assert patterns == {pattern.value for pattern in DispersionPattern}
    assert SCATTER_ONLY_READING in readings
    assert drift == {True, False}, "the drift caveat on only one side of SESSION_DRIFT_FACTOR"
    for opening in ("The single swing", "All 3 swings", "2 of the 6 swings"):
        assert any(c.startswith(opening) for c in caveats), opening
    for phrase in (
        "shots on this club was set aside",
        "shots on this club were set aside",
        " 1 was flagged automatically",
        " 2 were flagged automatically",
    ):
        assert any(phrase in c for c in caveats), phrase


def test_every_report_branch_has_recorded_text() -> None:
    """Each branch of the five reports, found by a sentence it prints, in at least one case: the
    recorded text is what gates `crates/core`'s renderers, and a branch no case reaches is a
    renderer nothing checks."""
    printed: dict[str, list[str]] = {key: [] for key in _CAREER_BRANCHES}
    for _, vector in _career_built():
        for key, text in vector["expected"]["reports"].items():
            group = key if key in printed else "club_profile_*"
            assert group != "club_profile_*" or key.startswith("club_profile_"), key
            printed[group].append(text)
    for key, phrases in _CAREER_BRANCHES.items():
        joined = "\n".join(printed[key])
        for phrase in phrases:
            assert phrase in joined, f"no case's {key} prints {phrase!r}"


def test_frozen_python_answers_a_career_case_only_under_its_own_engine() -> None:
    """The corpus report prints frozen Python's engine as the installed one, so a case asking
    under any other pair is a question for Rust's renderer, and the recorder says so."""
    from conformance_vectors import _run_career

    given = dict(_career_built()[0][1]["input"])
    given["versions"] = {"installed": ANALYSIS_VERSION + 1, "comparable_from": ANALYSIS_VERSION - 2}
    with pytest.raises(ValueError, match="answers only under"):
        _run_career(given)


@pytest.mark.skipif(
    not (_SESSIONS.is_dir() and _GOLFERS.is_dir()),
    reason="no data/processed/{sessions,golfers}/: the real career case is read from the captures",
)
def test_the_real_career_case_starts_from_the_real_storage_corpus() -> None:
    """The plan's call 2: the real case's corpus is the real storage vector's, copied, and
    `_career_real` raises unless that still equals `read_corpus` over `data/` and the mishit
    listing prints the same over the real trees as over the corpus."""
    from conformance_vectors import _career_real

    vector = _career_real()
    stored = conformance._read_json(conformance.VECTORS / "storage" / "corpus" / "real.json.gz")
    assert vector["id"] == "career/real/aaron"
    assert vector["input"]["corpus"] == stored["expected"]["corpus"]
    assert vector["input"]["bag"] is not None, "the real case carries the declared bag"
    assert vector["expected"]["bag_profile"]["clubs"], "the real golfer profiles no club"


def test_the_career_family_is_committed_in_the_sub_families_python_recorded() -> None:
    """Recorded once, by `regenerate --career-once`, at `career_version` 0 unless a Rust
    re-record has ledgered it since, and never at an engine version."""
    by_family: dict[str, list[Path]] = {}
    for path in conformance.career_vector_paths():
        family = path.relative_to(conformance.VECTORS / "career").parts[0]
        by_family.setdefault(family, []).append(path)
    assert set(by_family) == {"synthetic", "real"}, sorted(by_family)
    for paths in by_family.values():
        for path in paths:
            vector = conformance._read_json(path)
            name = vector["id"]
            assert name == conformance._vector_id(path), f"{path.name} names itself {name}"
            assert vector["provenance"]["kind"] == "career", name
            assert vector["provenance"]["oracle"] == "python", name
            assert vector["provenance"]["python_version"].startswith("3."), name
            assert "analysis_version" not in vector, f"{name} claims an engine version"
            assert isinstance(vector.get("career_version"), int), name
            # `list` prints every note to a console that may encode as cp1252 (M36 P2).
            vector["provenance"]["note"].encode("cp1252")
            if not vector["provenance"].get("rerecords"):
                assert vector["career_version"] == 0, name
    assert (conformance.VECTORS / "career" / "real" / "aaron.json.gz").is_file()


def test_check_defers_the_career_family_and_judges_no_staleness(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """Named and deferred, never run, and never STALE, even at a `career_version` frozen Python
    never had: the storage family's pin, for the family that shares its key."""
    vectors = _copy_career_family(tmp_path, monkeypatch)
    ahead = next(vectors.rglob("*.json"))
    document = json.loads(ahead.read_text(encoding="utf-8"))
    document["career_version"] = 99
    (ahead.parent / "ahead.json").write_text(json.dumps(document), encoding="utf-8")

    def never(vector: dict[str, Any]) -> dict[str, Any]:
        raise AssertionError(f"`check` ran career vector {vector.get('id')} through the engine")

    monkeypatch.setattr(conformance, "run_vector", never)
    assert conformance.main(["check"]) == 0
    out = capsys.readouterr().out
    count = len(conformance.career_vector_paths())
    assert f"{count} career vectors are `crates/analysis`' and `crates/core`'s" in out
    assert "STALE" not in out


def test_list_shows_a_career_vector_at_its_career_version(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """The storage family's pin, over the career family: each row at the vector's own version."""
    _copy_career_family(tmp_path, monkeypatch)
    versions = {
        conformance._vector_id(path): conformance._read_json(path)["career_version"]
        for path in conformance.career_vector_paths()
    }
    assert conformance.main(["list"]) == 0
    rows = [line for line in capsys.readouterr().out.splitlines() if line.startswith("career/")]
    assert rows, "`list` showed no career vector"
    assert len(rows) == len(versions), rows[:3]
    for row in rows:
        version = versions[row.split()[0]]
        assert isinstance(version, int), row
        assert re.search(rf"\s+v{version}\s+career\s", row), row


@pytest.mark.parametrize("what", ["a committed vector", "any file at all"])
def test_career_once_refuses_while_the_family_has_anything_in_it(
    what: str,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Once means once, `--storage-once`'s rule: refused on any file under `career/`, before the
    recorder is imported, so a refusal cannot reach `data/`, and it writes nothing."""
    if what == "a committed vector":
        vectors = _copy_career_family(tmp_path, monkeypatch)
    else:
        vectors = tmp_path / "spec" / "vectors"
        (vectors / "career").mkdir(parents=True)
        (vectors / "career" / "README").write_text("not a vector", encoding="utf-8")
        monkeypatch.setattr(conformance, "VECTORS", vectors)
    before = _snapshot(vectors)
    calls = _refuse_to_build(monkeypatch)

    assert conformance.main(["regenerate", "--career-once"]) == 2
    assert not calls, f"a refused `--career-once` still reached {calls}"
    assert _snapshot(vectors) == before
    message = capsys.readouterr().err
    assert "golf-core rerecord" in message
    assert "career-v<N>.json" in message


def test_career_once_records_an_empty_family_and_then_refuses(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """The one run writes what `build_career` returned, and the next run is refused by it."""
    vectors = tmp_path / "spec" / "vectors"
    vectors.mkdir(parents=True)
    monkeypatch.setattr(conformance, "VECTORS", vectors)
    recorded = vectors / "career" / "synthetic" / "only.json"
    builds: list[str] = []

    def build_career() -> list[tuple[Path, dict[str, Any]]]:
        builds.append("build_career")
        return [(recorded, {"id": "career/synthetic/only", "provenance": {"kind": "career"}})]

    fake = types.ModuleType("conformance_vectors")
    fake.build_career = build_career  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "conformance_vectors", fake)

    assert conformance.main(["regenerate", "--career-once"]) == 0
    assert recorded.exists()
    assert conformance.main(["regenerate", "--career-once"]) == 2
    assert builds == ["build_career"], "the second run reached the recorder"
