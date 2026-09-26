//! Rule-based ranked coaching tips. The Rust half of `src/golf_coach/feedback/`. [M22 P6]
//!
//! One module, [`rules`], and that is the whole of what ports: `feedback/coach.py` and
//! `feedback/conversation.py` are **sidecar**, not deferred (ADR-032 §8). The LLM call is one of
//! only two things ADR-030 keeps in Python on purpose, alongside MediaPipe pose, and
//! `docs/CONFORMANCE.md` already says the prose is unpinned — `FeedbackPayload.coaching` is `None`
//! on every one of the 21 vectors.
//!
//! # Why this is a crate and not a module of `analysis`
//!
//! ADR-008: modules never import each other, everything imports `contracts`. `analysis` may not
//! import `feedback`, which is the reason `analyze_swing_bundle` leaves `SwingBundleResult.feedback`
//! as `None` and `api/pipeline.py:1259` fills it in a line later — and therefore the reason
//! `conformance.py::run_vector` makes **two** calls rather than one. ADR-032 §1 asks cargo to enforce
//! that rule as a dependency edge, so this crate depends on `contracts` and on nothing else of ours;
//! `crates/core` is the shell that holds both halves, the way `api/` and `conformance.py` do.
//!
//! The consequence worth knowing: a vector recorded off the engine alone pins `"feedback": null` and
//! quietly tells a port to ship a results page with no coaching on it. M19 P1 shipped exactly that
//! and corrected it, which is why [`rules::build_feedback`]'s output is gated by the engine vectors
//! rather than by a stage.

pub mod rules;
