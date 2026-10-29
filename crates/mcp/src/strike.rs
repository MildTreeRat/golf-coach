//! One golfer's graded strike profile, in the shape an MCP tool returns. [M29 P15]
//!
//! **No Python twin, and none is coming.** M37's graded profile was written Rust first with no
//! Python behind it (`contracts::strike`'s module doc), so frozen Python's server has no tool here
//! and the recorded surface lists eleven (the plan's P8 finding 5). What this tool answers is held
//! by `spec/vectors/mcp/hand/`'s hand-worked strike case and `spec/vectors/mcp/rust/`, recorded once
//! by Rust (`oracle: "rust"`), rather than by an oracle.
//!
//! # The shape is `golf-core profile --json`'s
//!
//! One element of that verb's array, `{display_name, profile}`, with the `GradedProfile` whole
//! under `profile` (M37's handoff: "`golf-core profile --json` is the shape it reads"). So the
//! phone (through the bridge), the verb and a model read one shape, and nothing here re-words,
//! narrows or summarizes a grade. Its silences are the profile's own: a topic that cannot exist is
//! absent from `topics`, one too thin to grade carries a withheld share naming its counts, and a
//! blend with no graded topic is absent (`contracts::strike`'s "Absent, withheld, graded"), each
//! of which the tool's description tells a model how to read. A golfer with no shot photo answers
//! an empty profile, which is an answer and not a miss.
//!
//! # The same corpus and bag the verb reads
//!
//! [`career::resolve_golfer`], then `read_corpus` under [`career::versions`] (so the profile pools
//! what the career and club tools pool) and the golfer's `BagStore` entry, then
//! `golf_core::profile::run_profile`, the one join of `analysis::strike`'s grades and `feedback`'s
//! list (M37 P9). That is exactly `golf_core::reports::answer`'s `profile` arm, and
//! `tests/tools.rs` holds the two to one answer over every tree the family's strike cases read.

use std::path::Path;

use contracts::strike::GradedProfile;
use contracts::ContractError;
use golf_core::profile::run_profile;
use serde::Serialize;
use storage::bag_store::BagStore;
use storage::corpus::read_corpus;

use crate::career;
use crate::query::{self, NotFound};

/// What `get_strike_profile` returns: one element of `golf-core profile --json`'s array.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrikeProfileView {
    pub display_name: String,
    pub profile: GradedProfile,
}

/// `get_strike_profile`'s miss: `career::missing_golfer_profile`'s sentence and hint, because the
/// miss is the same one (a name nobody on file answers to) and so is what to do about it.
pub fn missing_golfer_strike(player: &str) -> NotFound {
    query::missing(
        format!(
            "No golfer matching {} is registered.",
            pyfmt::str_repr(player)
        ),
        "Names are matched loosely (case and punctuation are folded), so this means nobody by \
         that name has swings on file yet.",
    )
}

/// One golfer's graded profile. `None` if the golfer is unknown; `Some(Err)` only when the
/// assembled profile fails its own `Validate`, which `run_profile` answers for a builder bug rather
/// than panicking (M37 P9's finding 4), and which no recorded tree reaches.
pub fn strike_profile(
    sessions_dir: &Path,
    golfers_dir: &Path,
    player: &str,
) -> Option<Result<StrikeProfileView, ContractError>> {
    let golfer = career::resolve_golfer(golfers_dir, player)?;
    let corpus = read_corpus(sessions_dir, &golfer.player_id, career::versions());
    let bag = BagStore::new(golfers_dir).get(&golfer.player_id);
    Some(
        run_profile(&corpus, bag.as_ref()).map(|profile| StrikeProfileView {
            display_name: golfer.display_name,
            profile,
        }),
    )
}
