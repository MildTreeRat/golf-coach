//! The swing analysis engine, ported. [M22 P3]
//!
//! The Rust half of `src/golf_coach/analysis/`, and the fourth crate in this workspace. What it
//! has to reproduce is not a design but an *answer*: `scripts/conformance.py::run_vector` is the
//! specification (M19), `spec/vectors/` is the oracle, and ADR-032 §2 gives every stage of the
//! port its own committed gate so that a divergence is caught by the phase that caused it rather
//! than by a whole-bundle diff at the end.
//!
//! Module names carry over one for one from the Python package, for the reason
//! [`contracts`]'s crate doc gives: every phase of this port gets diffed against its source by eye
//! at least once, and a port that renames while it translates cannot be.
//!
//! # `pyfmt` is first, and that ordering is the phase
//!
//! [`pyfmt`] is the whole of P3 and nothing calls it yet. ADR-032 §3 found **three** portability
//! edges between CPython and Rust, and all three land on the *strings* `docs/CONFORMANCE.md` §3
//! compares exactly rather than on the floats it compares within `RTOL`:
//!
//! 1. `round()` is half-to-even in Python and half-away-from-zero in Rust — forty-six sites, and
//!    the seventeen that round a **frame index** decide which frame a checkpoint is measured on.
//! 2. `%g` and `.Nf` are interpolated into `CheckpointScore.message` and `Measurement.detail`.
//! 3. A stable sort over an **insertion-ordered** dict decides which *name* a sentence names.
//!
//! Solving them in one module before anything calls it is what stops each one being re-solved,
//! differently, at the seven call sites downstream. The gate is `spec/vectors/format/` — CPython's
//! own answers, 2,681 cases at P3 and grown since, now run by `crates/pyfmt/tests/format.rs` — and
//! it exists for the same reason M20 P0's audio vectors did: the first line of an implementation
//! should have something to fail against.
//!
//! It is no longer a module of this crate. M34 P1 moved it to `crates/pyfmt`, unchanged, because
//! `feedback` and `screen` need the same edges and may not depend on `analysis`. Every
//! [`pyfmt`] below names that crate, and this one does not re-export it.
//!
//! # P4 is the geometry, and the first phase with a caller
//!
//! [`smoothing`], [`phases`], [`measure`] and the reachable half of [`stats`] — the four modules
//! `docs/CONFORMANCE.md`'s stage table calls this phase's, gated by the `smoothed`, `phases` and
//! `measure` stages on all 21 vectors. They are the engine's whole *measuring* half: numbers with
//! no band in sight. Judging is P5's, and the split is the Python's — it is what lets a new metric
//! be measured across the corpus before a band for it exists.
//!
//! Two of [`pyfmt`]'s three edges have their first caller here ([`measure::address_sample_bounds`]
//! rounds a frame count, [`measure::tempo_timings`] formats one into a sentence), and [`phases`]
//! found a fourth that ADR-032 §3 does not name: Python's `max` returns the first maximum where
//! Rust's returns the last, which on a repeated wrist `y` is a different impact frame.
//!
//! # P5 judges, P5b places, and the split is the Python's
//!
//! [`benchmarks::store`], [`benchmarks::distributions`] and [`checkpoints`] are the **judging**
//! half — a number in, a verdict and a sentence out, gated by the `checkpoints` stage (P5).
//! [`trajectory`], [`pivot`], [`benchmarks::joint`], [`benchmarks::trajectory`] and [`engine`]'s
//! three face-on measurement groups are the other thing a swing produces: quantities that are
//! **recorded and judged by nothing**, gated by the first three groups of the `measurements`
//! stage (P5b). No band, no `passed`, no path to `overall_score` — ADR-010 §2's firewall, and what
//! lets a new metric be measured across the corpus before a band for it exists.
//!
//! [`engine`] is the first module here that arrives half-written, and deliberately: P6 brings
//! `analyze_swing_bundle` and the rest of `_measurements`. Every one of the seven `measurements`
//! groups is a phase boundary, so a module that waited for all of them would be a module with no
//! gate until the end.
//!
//! # P8 flies the ball, and P8b joins it to the swing
//!
//! [`benchmarks::flight_model`], [`flight`] and [`spin_solve`] are the **outcome**'s forward half: the
//! published constants, the RK4 integrator over them, and the inverse problem of recovering a spin
//! from a printed carry. Gated by the `flight` stage's `flown.flight` on the five corpus vectors that
//! fly and its `resolved.spin.solution` on the eleven that solve, which is `tests/flight.rs` — the
//! seventh and last stage runner, and the first gate in this crate that reads a function's committed
//! *input* beside its output.
//!
//! The split is the Python's again, and `spin_solve`'s own docstring argues for it: the forward model
//! and the inverse over it stay apart so that the forward model's property — every number re-flown
//! rather than remembered — stays obvious.
//!
//! [`shot_measure`], [`flight_infer`] and [`flight_measure`] are the **join** (P8b): the shot's seven
//! derived tiles, ADR-027 §Decisions 3 and 5's resolution of the two launch conditions the screen did
//! not print, and the layer that decides which of the six flight numbers becomes a `Measurement`.
//! They complete [`engine`]'s `measurements` — the `shot` and `flight` groups — and put the
//! `flight_unscored` entries onto `SwingResult::unscored`, which is what takes the fifteen corpus
//! vectors into `crates/core`'s whole-bundle gate. **All 21 now run end to end.**
//!
//! P8 also found the first portability edge that is not a string. [`pyfmt::hypot`] is CPython's
//! `math.hypot`, ported because one ulp of the three-argument norm flips
//! [`benchmarks::flight_model::AeroCoefficients::clamped`] at the shoulder `spin_solve` constructs;
//! and `Cargo.toml` records the second half of it, which is that `serde_json` needs
//! `float_roundtrip` or the port's own oracle is read a ulp off.
//!
//! # What this crate may not reach for
//!
//! ADR-030 §1's stdlib-only invariant is inherited here as *no numeric library in the scoring
//! path*, and ADR-032 §6 settles the one question it raises: `serde` is not one, because it
//! computes nothing. `crates/trigger`'s `rustfft` stays the single sanctioned numeric exception in
//! this workspace, for the FFT the Python detector needed numpy for. Nothing in here gets one.

pub mod alignment;
pub mod benchmarks;
pub mod checkpoints;
pub mod engine;
pub mod flight;
pub mod flight_infer;
pub mod flight_measure;
pub mod measure;
pub mod phases;
pub mod pivot;
pub mod scoring;
pub mod shot_measure;
pub mod smoothing;
pub mod spin_solve;
pub mod stats;
pub mod trajectory;

#[cfg(test)]
pub(crate) mod testing;
