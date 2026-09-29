//! `golf-pose` — one clip through the real pool, written to a real file. [M23 P6]
//!
//! ```text
//! golf-pose run <clip> --out <dir> [--camera-id ID] [--variant V] [--python PATH] [--force]
//! golf-pose sweep <clip>... [--workers N] [--variant V] [--python PATH]      # M23 P8
//! ```
//!
//! The gate ADR-033's "Consequences" asks for, and the `golf-core run` move made one milestone
//! later: a vector family was declined for pose because its true input is a 4K `.MOV` that cannot be
//! committed and a keypoints-only family would be 239 MB before gzip, so what stands in for it is a
//! binary plus a diff against the 30 keypoints files already on disk. P7 replays the whole corpus
//! through this; this is the one clip that proves the path exists.
//!
//! **It prints what it did**, which is the `golf-capture list` precedent rather than the
//! `golf-core run` one: nothing parses this output, the consumer is a human reading a run, and the
//! numbers it prints — frames, wall clock, effective fps — are the measurement the phase was for. A
//! machine format with no reader is a schema nobody validates; the day something needs to parse this
//! is the day it gets `--json`.
//!
//! # Why `--out` is required, and why an existing file is refused
//!
//! Because the obvious default is destructive here. A clip lives *in* its swing directory —
//! `data/processed/sessions/2026-08-23/11/face_on.21831919bc67.MOV` — beside the
//! `face_on.keypoints.json` that is **M23's only oracle**: 30 files, written by the Python pipeline,
//! and the sole thing that can say whether this sidecar agrees with it. Defaulting the output beside
//! the clip would make the natural first invocation overwrite the answer it was being checked
//! against, and `--force` exists so that replacing one is a thing someone typed rather than a thing
//! they discovered. §M30's clip trimming re-cuts those clips and invalidates all 30, which is
//! exactly why the plan orders P7 before it.
//!
//! # Why `sweep` exists, and why it is not `run` with a second clip
//!
//! [`pose::pool::DEFAULT_POOL_SIZE`] has to be a measurement, and a binary that poses one clip
//! cannot make it: N warm workers with one job between them measures nothing but N idle
//! interpreters. So `sweep` runs several clips through **one** pool of `--workers` workers and prints
//! the rate — which is the whole of what P8 needed the binary to gain, and what P7's finding said it
//! did not have.
//!
//! It is a second subcommand rather than a multi-clip `run` for a reason that is about the artifact
//! and not about taste: `run` names its output from the clip's leading dot-component, and three of
//! the six clips this phase measures are called `down_the_line.<sha>.MOV`, so a multi-clip `run`
//! would either need a per-clip `--out` or silently overwrite two thirds of what it wrote. `sweep`
//! writes **nothing** instead. P7 already proved the landmarks are the right landmarks over all 30
//! clips, so re-proving it here would put 100 MB of disk in the middle of a CPU measurement — and
//! P6's finding 4 asked for no batch mode on `run` for the same reason in reverse.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use pose::protocol::Job;
use pose::worker::Outcome;
use pose::{Config, Pool, PoolConfig};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("") {
        "run" => match Options::parse(&args[1..]) {
            Ok(options) => run(&options),
            Err(problem) => usage(&problem),
        },
        "sweep" => match Sweep::parse(&args[1..]) {
            Ok(options) => sweep(&options),
            Err(problem) => usage(&problem),
        },
        other => usage(&format!(
            "unknown command {other:?}; expected `run` or `sweep`"
        )),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("golf-pose: {problem}");
    eprintln!(
        "usage: golf-pose run <clip> --out <dir> [--camera-id ID] [--variant V] \
         [--python PATH] [--force]"
    );
    eprintln!("       golf-pose sweep <clip>... [--workers N] [--variant V] [--python PATH]");
    // 2 rather than 1, the way `golf-capture` separates a usage error from a failure to enumerate:
    // a caller scripting this can tell "I asked wrongly" from "the machine cannot pose".
    ExitCode::from(2)
}

/// Hand-parsed, as in the other two binaries in this workspace.
///
/// No `clap`: three optional flags and one positional do not earn a dependency, and ADR-030 §8's
/// stdlib rule is about the scoring path but the habit is the repo's. The day this grows a `--json`
/// and a subcommand tree is the day the trade changes.
struct Options {
    clip: PathBuf,
    out: PathBuf,
    camera_id: Option<String>,
    variant: Option<String>,
    python: Option<PathBuf>,
    force: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut clip: Option<PathBuf> = None;
        let mut out: Option<PathBuf> = None;
        let mut camera_id = None;
        let mut variant = None;
        let mut python = None;
        let mut force = false;

        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let mut value = |flag: &str| -> Result<String, String> {
                rest.next()
                    .cloned()
                    .ok_or_else(|| format!("{flag} needs a value"))
            };
            match arg.as_str() {
                "--out" => out = Some(PathBuf::from(value("--out")?)),
                "--camera-id" => camera_id = Some(value("--camera-id")?),
                "--variant" => variant = Some(value("--variant")?),
                "--python" => python = Some(PathBuf::from(value("--python")?)),
                "--force" => force = true,
                flag if flag.starts_with("--") => return Err(format!("unknown flag {flag:?}")),
                positional if clip.is_none() => clip = Some(PathBuf::from(positional)),
                extra => return Err(format!("unexpected argument {extra:?}")),
            }
        }

        Ok(Self {
            clip: clip.ok_or("no clip given")?,
            out: out.ok_or(
                "--out is required; a keypoints file written beside its clip would overwrite the \
                 committed one this is diffed against",
            )?,
            camera_id,
            variant,
            python,
            force,
        })
    }

    /// What names the artifact: the camera id if one was given, else the clip's leading name part.
    ///
    /// `face_on.21831919bc67.MOV` yields `face_on`, which is what this repo's swing directories
    /// already call that file — the sha fragment in the middle is `crates/capture`'s doing. It is a
    /// convenience for the file *name* only: the `camera_id` that reaches every
    /// `FrameKeypoints.camera_id` is `--camera-id` and nothing else, because inventing one would put
    /// a guess in an artifact rather than in a path.
    fn artifact_name(&self) -> Option<&str> {
        match &self.camera_id {
            Some(id) => Some(id.as_str()),
            None => self
                .clip
                .file_name()?
                .to_str()?
                .split('.')
                .next()
                .filter(|part| !part.is_empty()),
        }
    }
}

fn run(options: &Options) -> ExitCode {
    let Some(name) = options.artifact_name() else {
        return usage("cannot name the artifact from the clip; pass --camera-id");
    };
    let target = pose::keypoints_path(&options.out, name);
    if target.exists() && !options.force {
        eprintln!(
            "golf-pose: {} exists; pass --force to replace it",
            target.display()
        );
        return ExitCode::FAILURE;
    }
    // Absolute, because `Job::clip_path` is specified absolute (clause 2) and the worker runs with
    // the repo root as its working directory rather than this process's.
    let clip = match std::path::absolute(&options.clip) {
        Ok(clip) => clip,
        Err(e) => {
            eprintln!("golf-pose: {}: {e}", options.clip.display());
            return ExitCode::FAILURE;
        }
    };

    let config = PoolConfig {
        worker: Config {
            python: options.python.clone(),
            variant: options.variant.clone(),
            ..Config::default()
        },
        // One clip, one worker — not [`pose::pool::DEFAULT_POOL_SIZE`], which is 2. A second warm
        // interpreter for one job is 1.3 GB resident (P8's measurement) that poses nothing, and the
        // `sweep` subcommand below is where more than one worker is the point.
        size: 1,
        ..PoolConfig::default()
    };

    let started = Instant::now();
    let mut pool = match Pool::new(&config) {
        Ok(pool) => pool,
        Err(e) => {
            // Every variant of this names what it looked for or how long it waited, which is the
            // whole reason `PoseError` carries those rather than a message about them.
            eprintln!("golf-pose: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("interpreter  {}", pool.interpreter().display());
    println!("handshake    {:.2}s", started.elapsed().as_secs_f64());

    let job = Job {
        job_id: "golf-pose-1".to_string(),
        clip_path: clip.clone(),
        frame_range: None,
        camera_id: options.camera_id.clone(),
        pose_model_variant: options.variant.clone(),
    };
    println!("clip         {}", clip.display());
    if let Err(e) = pool.submit(job) {
        eprintln!("golf-pose: {e}");
        return ExitCode::FAILURE;
    }

    let posing = Instant::now();
    // `recv` and not `recv_timeout`: clause 6's deadlines are the worker's and the pool reports one
    // as a `WorkTimeout`, so a second timeout here would be a shorter one racing the honest one.
    let completed = match pool.results().recv() {
        Ok(completed) => completed,
        Err(_) => {
            eprintln!("golf-pose: the pool's drivers stopped without answering");
            return ExitCode::FAILURE;
        }
    };
    let elapsed = posing.elapsed();
    let _ = pool.shutdown();

    let keypoints = match completed.result {
        Ok(Outcome::Done(keypoints)) => keypoints,
        // A named fact about this clip, and never a partial file: ADR-010 §2 at the process boundary
        // means nothing is written when a clip could not be posed.
        // `{reason:?}` and not a `Display` added to `FailureReason` for this one caller: the variant
        // name is one underscore-transform from ADR-033 clause 5's wire spelling, and growing P2's
        // protocol surface for an error line in a dev binary is the wrong way round.
        Ok(Outcome::Failed { reason, detail }) => {
            eprintln!("golf-pose: {}: {reason:?} — {detail}", clip.display());
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("golf-pose: {e}");
            return ExitCode::FAILURE;
        }
    };

    report(&keypoints, completed.attempts, elapsed);
    match pose::writer::write(&keypoints, &target) {
        Ok(()) => {
            let bytes = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
            println!("wrote        {} ({bytes} bytes)", target.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("golf-pose: writing {}: {e}", target.display());
            ExitCode::FAILURE
        }
    }
}

/// The run's own numbers, which are why this binary prints at all.
///
/// Effective fps is reported beside the wall clock because it is the figure every decision in
/// ADR-033 clause 6 is derived from — `fps_estimate` is 9.2 and `MARGIN` is 3 — and a run that came
/// in at half that is the observation that would revise the default. `frames` is the decoded count
/// off the envelope rather than the acknowledged one, so a silently short decode shows up here as a
/// smaller number than the container claimed.
fn report(
    keypoints: &contracts::keypoints::KeypointsFile,
    attempts: u32,
    elapsed: std::time::Duration,
) {
    let frames = keypoints.frames.len();
    if let Some(clip) = &keypoints.clip {
        match (clip.width, clip.height, clip.fps) {
            (Some(w), Some(h), Some(fps)) => println!("source       {w}x{h} at {fps:.3} fps"),
            _ => println!("source       partly unrecorded"),
        }
        if let Some(sha) = &clip.source_sha256 {
            println!("sha256       {sha}");
        }
    }
    println!(
        "estimator    {}",
        keypoints.pose_estimator.as_deref().unwrap_or("unrecorded")
    );
    let seconds = elapsed.as_secs_f64();
    println!(
        "posed        {frames} frames in {seconds:.1}s ({:.1} fps), {attempts} attempt(s)",
        frames as f64 / seconds
    );
}

/// `sweep`'s options: several clips, one pool, and how wide it is. [M23 P8]
///
/// No `--out` and no `--camera-id`. The first because nothing is written; the second because
/// `camera_id` is copied into every frame and therefore into the digest, so leaving it unset keeps
/// the six clips comparable to each other without inventing an id for any of them — P6's rule that a
/// guessed `camera_id` never reaches an artifact, kept by there being no artifact.
struct Sweep {
    clips: Vec<PathBuf>,
    workers: usize,
    variant: Option<String>,
    python: Option<PathBuf>,
}

impl Sweep {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut clips = Vec::new();
        let mut workers = 1usize;
        let mut variant = None;
        let mut python = None;

        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let mut value = |flag: &str| -> Result<String, String> {
                rest.next()
                    .cloned()
                    .ok_or_else(|| format!("{flag} needs a value"))
            };
            match arg.as_str() {
                "--workers" => {
                    let given = value("--workers")?;
                    workers = given
                        .parse()
                        .map_err(|_| format!("--workers wants a number, not {given:?}"))?;
                }
                "--variant" => variant = Some(value("--variant")?),
                "--python" => python = Some(PathBuf::from(value("--python")?)),
                flag if flag.starts_with("--") => return Err(format!("unknown flag {flag:?}")),
                positional => clips.push(PathBuf::from(positional)),
            }
        }

        if clips.is_empty() {
            return Err("no clips given; a sweep of one clip would measure one idle worker".into());
        }
        if workers == 0 {
            return Err("--workers 0 would accept jobs nothing runs".into());
        }
        Ok(Self {
            clips,
            workers,
            variant,
            python,
        })
    }
}

fn sweep(options: &Sweep) -> ExitCode {
    let mut clips = Vec::with_capacity(options.clips.len());
    for clip in &options.clips {
        match std::path::absolute(clip) {
            Ok(clip) => clips.push(clip),
            Err(e) => {
                eprintln!("golf-pose: {}: {e}", clip.display());
                return ExitCode::FAILURE;
            }
        }
    }

    let config = PoolConfig {
        worker: Config {
            python: options.python.clone(),
            variant: options.variant.clone(),
            ..Config::default()
        },
        size: options.workers,
        // Every job is queued before the first result is read, so the bound is this run's size rather
        // than `DEFAULT_QUEUE_BOUND`'s two swings of latency. That default is about a golfer waiting;
        // a sweep is a batch with nobody waiting, and a refusal here would measure the bound instead
        // of the pool.
        queue_bound: clips.len(),
    };

    let started = Instant::now();
    let mut pool = match Pool::new(&config) {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("golf-pose: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("workers      {}", pool.size());
    println!("interpreter  {}", pool.interpreter().display());
    println!("handshake    {:.2}s", started.elapsed().as_secs_f64());

    let posing = Instant::now();
    for (n, clip) in clips.iter().enumerate() {
        let job = Job {
            job_id: format!("sweep-{}", n + 1),
            clip_path: clip.clone(),
            frame_range: None,
            camera_id: None,
            pose_model_variant: options.variant.clone(),
        };
        if let Err(e) = pool.submit(job) {
            eprintln!("golf-pose: {e}");
            return ExitCode::FAILURE;
        }
    }

    println!(
        "{:<30} {:>6} {:>8} {:>8} {:>17}",
        "clip", "frames", "done at", "attempts", "digest"
    );
    let mut frames = 0usize;
    let mut failures = 0usize;
    for _ in 0..clips.len() {
        let completed = match pool.results().recv() {
            Ok(completed) => completed,
            Err(_) => {
                eprintln!("golf-pose: the pool's drivers stopped without answering");
                failures += 1;
                break;
            }
        };
        let at = posing.elapsed().as_secs_f64();
        let name = label(&completed.clip_path);
        match completed.result {
            Ok(Outcome::Done(keypoints)) => {
                frames += keypoints.frames.len();
                println!(
                    "{name:<30} {:>6} {at:>7.1}s {:>8} {:>17}",
                    keypoints.frames.len(),
                    completed.attempts,
                    digest(&keypoints),
                );
            }
            // Counted and named, never silently dropped: a configuration that got its speed by
            // failing half the jobs would otherwise read as the fastest one.
            Ok(Outcome::Failed { reason, detail }) => {
                println!("{name:<30} FAILED {reason:?} — {detail}");
                failures += 1;
            }
            Err(e) => {
                println!("{name:<30} FAILED {e}");
                failures += 1;
            }
        }
    }
    let wall = posing.elapsed().as_secs_f64();
    let _ = pool.shutdown();

    println!(
        "\ntotal        {} clips, {frames} frames in {wall:.1}s ({:.2} fps), \
         {} worker(s), {failures} failure(s)",
        clips.len(),
        frames as f64 / wall,
        pool.size(),
    );
    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Enough of a clip's path to tell the corpus's clips apart: its swing directory and its file name.
///
/// The file name alone would not — `down_the_line.<sha>.MOV` is three of the thirty — and the whole
/// path is 70 characters of `data/processed/sessions/`.
fn label(clip: &std::path::Path) -> String {
    let name = clip.file_name().unwrap_or_default().to_string_lossy();
    match clip.parent().and_then(std::path::Path::file_name) {
        Some(swing) => format!("{}/{name}", swing.to_string_lossy()),
        None => name.into_owned(),
    }
}

/// A fingerprint of one reply, so that two configurations can be compared and not just timed.
///
/// P7 measured determinism *serially* — 30 clips through a one-worker pool, and the same container
/// posed three times on three fresh interpreters writing byte-identical files. It could not measure
/// it under **contention**, which is the property a pool size above 1 actually turns on, and that is
/// the one coverage gap P8 can close while it is here anyway: if every clip's digest is the same at
/// 1, 2 and 4 workers, then nothing about running four MediaPipe graphs at once changed a landmark
/// or mis-attributed a reply to the wrong job.
///
/// Over [`pose::writer::to_json`] and not the struct, so the thing hashed is exactly the artifact
/// `run` would have written. `DefaultHasher` and not sha256: this is compared between invocations of
/// **one built binary within one sweep**, never stored and never compared against a committed value,
/// so the stability `std` declines to promise across Rust releases is stability nothing here needs —
/// and a crate that depends on `contracts` alone does not gain a hashing dependency for a dev binary.
fn digest(keypoints: &contracts::keypoints::KeypointsFile) -> String {
    use std::hash::{Hash, Hasher};

    let Ok(json) = pose::writer::to_json(keypoints) else {
        return "unserializable".to_string();
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
