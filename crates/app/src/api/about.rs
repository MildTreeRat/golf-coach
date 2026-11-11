//! Which build of the core the phone is running. [M38 P4]
//!
//! The first thing across the bridge (P16's smoke test), and what the Settings tab shows. The version
//! keys are the ones a stored shot is judged by, so a phone's export can be read against the build
//! that wrote it.

use contracts::career::CAREER_VERSION;
use contracts::shot::SCREEN_PARSER_VERSION;
use contracts::swing::ANALYSIS_VERSION;

use crate::DEVICE;

/// The three version keys, the one device and this crate's version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutView {
    pub analysis_version: i64,
    pub screen_parser_version: i64,
    pub career_version: i64,
    pub device: String,
    pub crate_version: String,
}

/// What this build is, read from the constants rather than copied, so it cannot drift from them.
pub fn about() -> AboutView {
    AboutView {
        analysis_version: ANALYSIS_VERSION,
        screen_parser_version: SCREEN_PARSER_VERSION,
        career_version: CAREER_VERSION,
        device: DEVICE.to_string(),
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use contracts::capability::capability_for;

    use super::*;

    #[test]
    fn about_answers_the_three_version_keys_each_in_its_own_field() {
        let view = about();
        assert_eq!(view.analysis_version, ANALYSIS_VERSION);
        assert_eq!(view.screen_parser_version, SCREEN_PARSER_VERSION);
        assert_eq!(view.career_version, CAREER_VERSION);
    }

    /// Full Swing and nothing else (D3), and spelled as `devices.json` spells it: a device it does
    /// not declare is refused by every capability question a recorded shot goes on to ask.
    #[test]
    fn about_names_full_swing_as_the_one_device_and_contracts_declares_it() {
        let view = about();
        assert_eq!(view.device, "full_swing");
        capability_for(&view.device).expect("devices.json declares the phone's device");
    }

    #[test]
    fn about_answers_this_crates_version() {
        assert_eq!(about().crate_version, env!("CARGO_PKG_VERSION"));
    }
}
