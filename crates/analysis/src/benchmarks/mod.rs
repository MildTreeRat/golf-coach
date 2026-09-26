//! The benchmark data, and the two loaders the judging half reads. [M22 P5]
//!
//! The Rust half of `src/golf_coach/analysis/benchmarks/`. [`store`] answers *whether* a swing is
//! in band and [`distributions`] answers *where in the tour population it sits* — the split
//! ADR-010 §2 draws and the percentile addendum admits: scoring reads `ranges.json` and nothing
//! else, and the percentile is informational beside it.
//!
//! # The data crosses by `include_str!`, and there is one copy on disk
//!
//! ADR-032 §5. `ranges.json` and `golfdb_v1.json` are embedded at compile time **from the Python
//! package path** — `../../../src/golf_coach/analysis/benchmarks/` relative to this file — and
//! there is no copy under `crates/`. [ADR-022](../../../../docs/decisions/022-learned-artifacts-as-committed-data.md)
//! ships models as provenanced JSON precisely so a band has one authority, and a second copy in a
//! second language is the failure that invariant exists to prevent. `CLAUDE.md` says the same
//! thing about prose; a `const` in Rust would be prose with a type.
//!
//! Embedding also means no `REPO_ROOT` and no `importlib.resources`: the bytes are in the
//! binary, so the loaders below are infallible after the one parse and there is no I/O error
//! to thread through a signature that Python does not have one in.
//!
//! # What did not come across, and why
//!
//! `dataset_info()` and `DatasetInfo`, plus `BenchmarkRange`'s `source_date`/`added`: nothing on
//! `conformance.py::run_vector`'s path reads them. Their Python callers are `comparison.py` and
//! `tempo_trainer.py`, which are the lab, so no committed vector could gate a port of them — the
//! same rule P2 set for the `contracts/` registries and P4 applied to `stats`' career-mode half.
//! `Distribution.provenance` *is* here, because fourteen rows carry one and dropping a field that
//! the file contains would make the struct lie about the data rather than about a function.
//!
//! **P5b brought the next two.** [`joint`] places the six metrics as a *combination* and
//! [`trajectory`] places the whole motion against a fitted basis; both are recorded and never
//! judged, which is the firewall `contracts::placements` exists to state.
//!
//! **P8 brought the last one**, and it is the odd one out here: [`flight_model`] is not a benchmark at
//! all. Nothing in it was fitted and nothing in it places a swing against a population — it is
//! somebody else's wind-tunnel and test-range measurements, read by an integrator. It lives in this
//! package because the Python's does, on `clubs/catalogue.py`'s shape rather than [`joint`]'s, and
//! the port keeps the name so the two can be diffed by eye.
//!
//! Their `JointDatasetInfo` and `TrajectoryDatasetInfo` did not come, on the same rule as
//! `DatasetInfo` above: nothing on `run_vector`'s path reads either, so the `dataset` block stays in
//! the artifact unparsed. Nor did the **down-the-line** trajectory artifact —
//! [`trajectory::load_trajectory_model`] says where it lands and why it is not here.

pub mod distributions;
pub mod flight_model;
pub mod joint;
pub mod store;
pub mod trajectory;

pub use distributions::{load_distribution, Distribution};
pub use joint::{placement_for as joint_placement, JointPlacement};
pub use store::{resolve_range, ResolvedRange};
pub use trajectory::{trajectory_placement_for, TrajectoryPlacement};
