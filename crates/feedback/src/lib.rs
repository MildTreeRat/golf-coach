//! Rule-based ranked coaching tips. The Rust half of `src/golf_coach/feedback/`. [M22 P6]
//!
//! One module is a port, [`rules`], and that is the whole of what ports: `feedback/coach.py` and
//! `feedback/conversation.py` are **sidecar**, not deferred (ADR-032 §8). The LLM call is one of
//! only two things ADR-030 keeps in Python on purpose, alongside MediaPipe pose, and
//! `docs/CONFORMANCE.md` already says the prose is unpinned — `FeedbackPayload.coaching` is `None`
//! on every one of the 21 vectors.
//!
//! [`profile_rules`] (M37 P8) is the second module and has **no Python twin**: the strengths and
//! weaknesses list beside a golfer's per-club grades, ADR-034 §5.7's rule written Rust first. It is
//! here rather than beside the grades in `analysis` for the reason below, and it reads the career
//! layer's dispersion, never the grades.
//!
//! # Why this is a crate and not a module of `analysis`
//!
//! ADR-008: modules never import each other, everything imports `contracts`. `analysis` may not
//! import `feedback`, which is the reason `analyze_swing_bundle` leaves `SwingBundleResult.feedback`
//! as `None` and `api/pipeline.py:1259` fills it in a line later — and therefore the reason
//! `conformance.py::run_vector` makes **two** calls rather than one. ADR-032 §1 asks cargo to enforce
//! that rule as a dependency edge, so this crate depends on `contracts` and on nothing else of ours
//! but `pyfmt` (M34 P1), which depends on nothing of ours; `crates/core` is the shell that holds both
//! halves, the way `api/` and `conformance.py` do. `tests/profile_rules.rs` reads every workspace
//! member's manifest and holds this crate to those two (M37 P8), because the profile's list must
//! never reach a grade or a pose number through `analysis`.
//!
//! The consequence worth knowing: a vector recorded off the engine alone pins `"feedback": null` and
//! quietly tells a port to ship a results page with no coaching on it. M19 P1 shipped exactly that
//! and corrected it, which is why [`rules::build_feedback`]'s output is gated by the engine vectors
//! rather than by a stage.

pub mod profile_rules;
pub mod rules;
