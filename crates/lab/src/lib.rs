//! `golf-lab`: the lab CLI in Rust. The scripts in front of `src/golf_coach/api/pipeline.py`
//! (`analyze_bundle.py`, `reanalyze.py`, `analyze_swing.py`, `import_shot_screens.py`,
//! `simulate_flight.py`, and `conformance_vectors.py`'s corpus builder) and the pipeline behind
//! them. [M29 P20]
//!
//! ADR-035 clause 5 makes this M29's port of the lab: the bundle pipeline (strike audio, pose, the
//! shot lookup and re-read, swing selection, the engine, the writers, coaching) and every lab verb,
//! over the Rust crates that already hold each piece. A shell like `golf-core` (the M29 plan's D13):
//! it may hold any library crate, and nothing depends on it (`tests/edges.rs`). It differs from
//! `golf-core` in what it may reach: the lab's I/O edges, ffmpeg (D17), `crates/pose`'s sidecar and
//! `crates/ocr`'s reader with its `ort`, which M38's phone never links and so `golf-core` must not.
//!
//! **Built a phase at a time, and empty at the first.** P20 made the crate, the binary and the
//! strike cache's shape and store (`contracts::audio`, `storage::audio_io`), held by
//! `tests/audio.rs` to every stored file. Each module after it is named here when it lands:
//!
//! - [`ffmpeg`] (P21): a clip's audio as mono PCM and its video's start, `audio/ffmpeg.py`, with
//!   frozen Python's arguments and the lab's one ffmpeg finder, and `tests/ffmpeg.rs` holding the
//!   branches no stored clip reaches on clips it makes;
//! - [`audio`] (P21): `audio_for`, the strike cache's rule and its notes, over
//!   `trigger::detect_strikes` linked in, and `tests/strikes.rs` holding a forced detection over
//!   every stored clip to the file the store holds;
//! - [`pose`] (P22): `keypoints_for`, the pose cache's rule and its notes, with `crates/pose`'s pool
//!   started only when a view needs posing, which makes the lab that crate's first caller.
//!   `tests/sidecar.rs` drives every branch against `crates/pose`'s stub worker, and
//!   `tests/keypoints.rs` holds every stored view to a cache hit that spawns nothing, and one forced
//!   run of the real worker to the stored landmarks.
//! - [`shot`] (P23): `_shot_for`, the shot store's rule and its notes, with a stored shot an older
//!   parser read re-read from the bundle's photo through `crates/ocr`'s reader and stamped by
//!   `crates/screen`'s. `tests/shot_lookup.rs` drives every branch with a reader that replays a
//!   committed screen vector's boxes, and `tests/reread.rs` holds every stored bundle's shot to the
//!   store as P29 wrote it: stamped by this parser and served with no photo read. Until P29 it
//!   re-read each photo in a dry run and held the re-read to frozen Python's stored shot. Since M42
//!   P14 it also reads a Live Photo clip's grid ([`shot::read_clip_grid`], no Python twin): one
//!   frame written as a PNG and read as a still is, which `tests/full_swing_clips.rs` holds to the
//!   five grid clips' screens with the models.
//! - [`import`] (P24): `golf-lab import-photos`, `import_shot_screens.py`'s bulk import into the
//!   shot store through [`shot::import_photo`], and `--into-session`'s photo entries, each analysed
//!   by `golf-core run-shot --session`'s own code. `tests/import_photos.rs` drives both against a
//!   replaying reader, and `tests/import_real.rs` runs the binary over a stored photo with the models.
//!   Since M42 P14b it also holds `golf-lab import-video` ([`import::import_video`], no Python twin):
//!   a Live Photo's clip imported on its own, its grid frame as the photo and its replay read for the
//!   impact offset ([`shot::read_impact_video`]), which `tests/import_video.rs` drives with a fake
//!   decoder.
//! - [`pipeline`] (P25): `analyze_swing_dir` up to the engine call, pose, strikes, the swing
//!   selection (`golf_core::auto_windows`, which the selection family gates), the shot, the golfer's
//!   handedness and the club's loft assembled into `golf_core::VectorInput` with every note in
//!   Python's order, and [`pipeline::corpus_input`], the cut a committed corpus vector holds.
//!   `tests/assemble.rs` drives its branches on scratch bundles, and `tests/inputs.rs` is the M29
//!   plan's D20 gate: every stored bundle assembled in a dry run and held to its committed corpus
//!   vector's input, the re-read shot's declared paths aside.
//! - [`analyze`] (P26): `analyze_swing_dir` from the engine call on, through `golf_core::run_bundle`:
//!   the pipeline's notes, the loft repair, coaching, `analysis.json` (frozen Python's reasons only,
//!   the plan's C6) and `analysis.state.json` (`record_state`), with [`coach`], the coaching
//!   paragraph from P17's Python entry over `golf-mcp`, and [`report`], `analyze_bundle.py`'s
//!   summary and `--list-swings`. `golf-lab analyze` is the verb. `tests/analyze.rs` drives it on
//!   scratch bundles with a coach that answers in the test, and `tests/bundle/mod.rs` is the scratch
//!   bundle it shares with `tests/assemble.rs`.
//! - [`reanalyze`] (P27): `golf-lab reanalyze`, `reanalyze.py`'s staleness walk over the bundle
//!   store with [`analyze`] behind it, and photo entries re-run through `golf-core run-shot
//!   --session`'s own code after the shot lookup. `tests/reanalyze.rs` drives it on scratch
//!   bundles and photo entries.
//! - [`analyze_keypoints`] (P27): `golf-lab analyze-keypoints`, `analyze_swing.py`'s spine over one
//!   keypoints file and its report, sharing [`report`]'s checkpoint and tip blocks.
//! - [`flight`] (P28): `golf-lab flight`, `simulate_flight.py`: typed conditions, the validation
//!   gate, and every stored shot joined to its swing (`storage::flight_inputs::read_flight_inputs`)
//!   and flown, over `analysis`' integrator, solve, caveats and printed comparison. `tests/flight.rs`
//!   holds the typed and gate reports to the frozen script's text and drives the stored modes on a
//!   scratch store; one ignored case reads `data/`.
//!
//! - [`build_vectors`] (P30): `golf-lab build-vectors`, `conformance_vectors.py`'s corpus builder,
//!   its stage vectors and the real storage corpus, cut from `data/` and recorded by Rust: each
//!   verified as its twin verified it, and written only where no file is, so a committed vector is
//!   refused by name. `tests/build_vectors.rs` builds a scratch bundle's three and refuses them on a
//!   second run, and holds every stored bundle's to the committed vectors, the re-read shot aside.
//! - [`import_export`] (P31): `golf-lab import-export`, no Python twin: the phone's export, a zip or
//!   a directory of the stored layout itself (the M29 plan's D16), merged into the stores file by
//!   file. New files are copied, identical ones skipped, a shot and a photo entry deduped by photo,
//!   and one conflict refuses the whole import (C11). `tests/import_export.rs` builds a phone's
//!   photo-only session with the real writers and imports it, again, conflicting and zipped, ending
//!   on a corpus `read_corpus` reads as the phone's.
//!
//! - [`read_impact`] (M42 P13): `golf-lab read-impact`, no Python twin: the impact offset off a
//!   Live Photo clip's replay of Full Swing's screen, through [`ffmpeg`]'s frames and
//!   `crates/impact`'s reader, one line a clip, with review images of what each reading came from.
//!
//! **No render** (the plan's C4): no verb here writes `aligned.mp4`.

use std::path::{Path, PathBuf};

pub mod analyze;
pub mod analyze_keypoints;
pub mod audio;
pub mod build_vectors;
pub mod coach;
pub mod ffmpeg;
pub mod flight;
pub mod import;
pub mod import_export;
pub mod pipeline;
pub mod pose;
pub mod read_impact;
pub mod reanalyze;
pub mod report;
pub mod shot;

/// `Path.name`, for the log lines and notes that name a file the way Python's f-strings do.
pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The repository, found from this crate's source as `golf-ocr` and `golf-core` find theirs, so the
/// models under it are found from any working directory.
pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/lab sits two directories below the repository root")
        .to_path_buf()
}
