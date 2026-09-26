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
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

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
    """
    stale = []
    for name, expected in conformance.export_schemas().items():
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
    "profiles.json",  # ADR-014: OCR device profiles, and OCR stays Python (tier 4)
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


# --------------------------------------------------------------------------- P2: the vectors


def test_there_are_vectors_at_all() -> None:
    """A regeneration that silently produced nothing would make every test below vacuous."""
    ids = _vector_ids()
    assert any(i.startswith("synthetic/") for i in ids), "no synthetic vectors"
    assert any(i.startswith("corpus/") for i in ids), "no corpus vectors"


@pytest.mark.parametrize("path", conformance.engine_vector_paths(), ids=conformance._vector_id)
def test_each_vector_still_conforms(path: Path) -> None:
    """The oracle, run against itself — a port's `check` and this are the same comparison.

    A failure here is one of two things and the message cannot tell them apart, which is correct:
    either the engine changed and the vectors need regenerating, or the engine changed and should
    not have. `ANALYSIS_VERSION` is what distinguishes them, and the next test is what asks.
    """
    vector = conformance._read_json(path)
    diffs = conformance.compare_results(vector["expected"], conformance.run_vector(vector))
    assert not diffs, "\n".join(["this build disagrees with the committed vector:"] + [
        f"  {d}" for d in diffs[:10]
    ])


@pytest.mark.parametrize("path", conformance.engine_vector_paths(), ids=conformance._vector_id)
def test_each_vector_was_recorded_at_the_current_engine_version(path: Path) -> None:
    """A vector from an older engine certifies a port against answers this repo has retracted.

    Regenerating is the fix, and it is deliberately not automatic: an `ANALYSIS_VERSION` bump is
    supposed to be a moment where someone looks at what moved.
    """
    vector = conformance._read_json(path)
    assert vector["analysis_version"] == ANALYSIS_VERSION, (
        f"{path.name} was recorded at v{vector['analysis_version']}, engine is "
        f"v{ANALYSIS_VERSION} — run `python scripts/conformance.py regenerate`"
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
    """
    path = next(p for p in conformance.engine_vector_paths() if "synthetic" in p.as_posix())
    vector = conformance._read_json(path)
    produced = json.loads(json.dumps(conformance.run_vector(vector)))
    assert not conformance.compare_results(vector["expected"], produced)


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
    assert not missing, (
        f"no stage vector for {missing} — run `python scripts/conformance.py regenerate "
        f"--stages-only`"
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
def test_each_stage_vector_was_recorded_at_the_current_engine_version(path: Path) -> None:
    """Same rule as the engine family, and it bites earlier.

    A stale *bundle* vector certifies a finished port against retracted answers. A stale *stage*
    vector does it four phases sooner, to a port that then builds everything after it on top —
    which is why `conformance.py check` reads these versions even though it defers running them.
    """
    stated = conformance._read_json(path)["analysis_version"]
    assert stated == ANALYSIS_VERSION, (
        f"{path.name} was recorded at v{stated}, engine is v{ANALYSIS_VERSION} — run "
        f"`python scripts/conformance.py regenerate --stages-only`"
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
    """
    vector = conformance._read_json(path)
    engine = conformance._read_json(_engine_by_id()[vector["provenance"]["derived_from"]])
    diffs = conformance.compare_results(vector["stages"], conformance.run_stages(engine))
    assert not diffs, "\n".join(
        ["this build disagrees with the committed stage vector:"] + [f"  {d}" for d in diffs[:10]]
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
    """Five vectors, and a port that reads the table can solve every formatting edge found so far.

    Discovery rather than a listing, the same choice the rest of this file makes — but the reason
    here is sharper: `check` *defers* this family to `cargo test`, so the Python suite is the only
    thing that would notice it going missing, and a `pyfmt` with no table under it is exactly the
    "gated by review" state ADR-032 §2 exists to forbid.

    **Five families for four edges**, because ADR-032 §3 names three and M22 P5 found a fourth:
    `str(x)` on a float, which an f-string with no format spec reaches and which Rust's `{}`
    renders under different rules. `rounding` covers edge 1 in both its arities. (M22 P4 found a
    fifth edge — Python's `max` returns the first maximum — which is not CPython *formatting* and
    is gated by the stage vectors rather than here.)
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
    }, f"the format family is {sorted(by_id)} — run `regenerate --format-only`"
    for name, vector in by_id.items():
        assert vector["provenance"]["kind"] == "format", name
        assert vector["cases"], f"{name} records no cases"


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
