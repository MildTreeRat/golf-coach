//! What this crate itself binds: the init function and nothing else. Every call the app makes is in
//! `app::api` (`crates/app/src/api/`), which `flutter_rust_bridge.yaml` also reads, because a body
//! written here could not be tested without a bridge (the M38 plan's C1 and rule 7).

/// Runs once inside `RustLib.init()`, before any other call.
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
