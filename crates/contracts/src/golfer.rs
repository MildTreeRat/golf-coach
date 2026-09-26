//! `contracts/golfer.py`'s one reachable member. [M22 P2]
//!
//! The Python module also holds `Golfer`, `GolferStore` and `slugify` — identity, which the engine
//! never sees. `analyze_swing_bundle` takes a bare [`Handedness`] and nothing else from here
//! (ADR-032 §8), so that is what crosses.

use serde::{Deserialize, Serialize};

use crate::Validate;

/// Which side the golfer swings from — the frame of reference for every signed metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    Right,
    Left,
}

impl Handedness {
    /// The wire name, for a sentence rather than for a serializer. [M22 P8b]
    ///
    /// `flight_infer::infer_spin_axis` formats it into the provenance string on a resolved spin axis
    /// — `"+2.5 deg printed, read right-handed"` — which `docs/CONFORMANCE.md` §3 compares exactly.
    /// The same need [`crate::intent::ClubCategory::as_str`] exists for, and the same answer: a
    /// serializer round trip would allocate and quote, and a second hand-written table would drift.
    pub fn as_str(self) -> &'static str {
        match self {
            Handedness::Right => "right",
            Handedness::Left => "left",
        }
    }
}

impl Validate for Handedness {
    fn validate(&self) -> Result<(), crate::ContractError> {
        Ok(())
    }
}
