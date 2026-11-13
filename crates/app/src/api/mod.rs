//! What crosses to Dart, and nothing else (the M38 plan's rule 7, spelled out in the crate doc).
//!
//! `flutter_rust_bridge_codegen` reads this module (`rust_input: crate::api,app::api`, C1) and binds
//! every `pub` item in it, one Dart file per row of C2's table. So a helper never goes here, however
//! small: it would become a function Dart can call.

pub mod about;
pub mod bay;
pub mod drill;
pub mod export;
pub mod history;
pub mod profile;
pub mod record;
pub mod review;
pub mod setup;
