//! The three ONNX models, the manifest that pins them, and the sessions opened from them.
//! [M29 P1]
//!
//! No Python twin. Frozen PaddleOCR downloads its weights into `~/.paddlex/official_models/` on
//! first use and runs whatever is there; this reader runs ONNX conversions of three of those
//! models (the M29 plan's D5), and trusts none of them until it has hashed it. The conversions are
//! ~145 MB of third-party weights, so they are not committed: they live in gitignored
//! `data/models/ocr/`, and `crates/ocr/models.json` is committed in their place, with each file's
//! SHA-256, its source model and the constants its pre- and postprocessing need (D6).
//!
//! # Why the digest is checked on the bytes the runtime loads
//!
//! [`Models::open`] reads each file once, hashes those bytes and hands **the same bytes** to ONNX
//! Runtime (`commit_from_memory`), rather than hashing the file and then letting the runtime open
//! it by path. A file swapped between the two reads would otherwise load unverified. The price is
//! holding one model in memory while its session is built, which on the lab's desktop is nothing.
//!
//! # Why a session is checked against the manifest too
//!
//! A digest says the file is the one that was converted; it does not say the manifest describes
//! it. So every session's input and output are compared with the manifest's names and shapes, and
//! the dictionary's line count with the manifest's, and a disagreement is refused by name. The
//! one cross-check that cannot be made at runtime without the models, that the recognition model
//! has exactly `dict.entries + 2` classes (CTC's blank before the dictionary and `use_space_char`'s
//! space after it), is pinned by `tests/models.rs` against the manifest alone.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ort::session::Session;
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The manifest, as committed. Parsed once by [`Manifest::committed`].
pub const MANIFEST_JSON: &str = include_str!("../models.json");

/// Where the converted models live, relative to the repo root: gitignored, and written only by the
/// one-time conversion the M29 plan's P1 finding records.
pub const DEFAULT_DIR: &str = "data/models/ocr";

/// One of the three models the reader runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Model {
    /// Text detection: a probability map of where text is (P4).
    Det,
    /// Textline orientation: whether a cropped line is upside down (P6).
    Textline,
    /// Recognition: a cropped line's characters, CTC-decoded (P5).
    Rec,
}

impl Model {
    /// In the order [`Models::open`] loads them, which is the order a photo passes through them
    /// apart from orientation sitting between detection and recognition.
    pub const ALL: [Model; 3] = [Model::Det, Model::Textline, Model::Rec];

    /// The model's key in `models.json`.
    pub fn key(self) -> &'static str {
        match self {
            Model::Det => "det",
            Model::Textline => "textline",
            Model::Rec => "rec",
        }
    }
}

/// `crates/ocr/models.json`. Only what code reads is typed; the `_comment`, `frozen`,
/// `conversion` and per-section `from` strings are for the reader of the file, as
/// `crates/contracts/devices.json`'s are.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub models: ModelSet,
}

#[derive(Debug, Deserialize)]
pub struct ModelSet {
    pub det: Det,
    pub textline: Textline,
    pub rec: Rec,
}

/// What every model entry carries: the file, its digest, and what the graph looks like.
#[derive(Debug, Deserialize)]
pub struct Pinned {
    /// The Paddle model's name, which is also its directory under [`DEFAULT_DIR`].
    pub name: String,
    /// Relative to the models directory, with `/` separators.
    pub file: String,
    /// Lower-case hex SHA-256 of the ONNX file.
    pub sha256: String,
    /// The ONNX opset the converter settled on (it raises the requested 7 to what a graph needs).
    pub opset: u32,
    pub source: Source,
    pub input: TensorSpec,
    pub output: TensorSpec,
}

/// The Paddle model a conversion came from, so a reconversion can be checked against the same
/// weights.
#[derive(Debug, Deserialize)]
pub struct Source {
    pub dir: String,
    pub pdiparams_sha256: String,
}

/// A graph input or output as ONNX Runtime reports it: `-1` for a dynamic dimension.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TensorSpec {
    pub name: String,
    pub shape: Vec<i64>,
}

impl fmt::Display for TensorSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:?}", self.name, self.shape)
    }
}

/// `NormalizeImage`: `(pixel * scale - mean) / std` per channel. Kept as the file writes it, in
/// `f64`. PaddleX folds it into `scale / std` and `-mean / std` in `f64` and applies both in
/// `f32`, one rounding each, which [`Normalize::folded`] gives `crate::det::det_input` and
/// `crate::cls::cls_input` (bit-exact on the bay photos, M29 P4 and P6).
#[derive(Debug, Deserialize)]
pub struct Normalize {
    pub scale: f64,
    pub mean: [f64; 3],
    pub std: [f64; 3],
    pub order: String,
    pub img_mode: String,
}

impl Normalize {
    /// PaddleX's fold, `alpha = scale / std` and `beta = -mean / std` per channel, worked in `f64`
    /// as Python's floats are and narrowed to `f32`, as NumPy 2 narrows a Python float that
    /// multiplies or is added to a `float32` array. A pixel then becomes `f32(pixel) * alpha +
    /// beta`, the product rounded before the sum (`*=`, then `+=`). The detector's and the
    /// textline classifier's `NormalizeImage` both come to this (M29 P4 and P6).
    pub fn folded(&self) -> ([f32; 3], [f32; 3]) {
        (
            std::array::from_fn(|c| (self.scale / self.std[c]) as f32),
            std::array::from_fn(|c| (-self.mean[c] / self.std[c]) as f32),
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct Det {
    #[serde(flatten)]
    pub pinned: Pinned,
    pub normalize: Normalize,
    pub resize: DetResize,
    pub postprocess: DbPostprocess,
}

/// `DetResizeForTest`'s limits, as the pipeline sets them.
#[derive(Debug, Deserialize)]
pub struct DetResize {
    pub limit_side_len: u32,
    pub limit_type: String,
    pub max_side_limit: u32,
}

/// `DBPostProcess`'s thresholds, as the pipeline sets them.
#[derive(Debug, Deserialize)]
pub struct DbPostprocess {
    pub name: String,
    pub thresh: f64,
    pub box_thresh: f64,
    pub unclip_ratio: f64,
    pub max_candidates: usize,
}

#[derive(Debug, Deserialize)]
pub struct Textline {
    #[serde(flatten)]
    pub pinned: Pinned,
    pub normalize: Normalize,
    pub resize: TextlineResize,
    /// Class index to label: `0_degree`, `180_degree`.
    pub labels: Vec<String>,
    pub batch_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct TextlineResize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct Rec {
    #[serde(flatten)]
    pub pinned: Pinned,
    /// `RecResizeImg`'s `[channels, height, width]`.
    pub image_shape: [u32; 3],
    pub img_mode: String,
    pub dict: Dictionary,
    pub score_thresh: f64,
    pub batch_size: usize,
}

/// The recognition model's character dictionary: a second pinned file beside the model.
#[derive(Debug, Deserialize)]
pub struct Dictionary {
    pub file: String,
    pub sha256: String,
    /// Lines in the file. The model's classes are this plus two (blank and space).
    pub entries: usize,
}

impl Manifest {
    /// The committed manifest, parsed once. A committed file that does not parse is a bug in this
    /// change, not a condition to handle, and `tests/models.rs` parses it on every run.
    pub fn committed() -> &'static Manifest {
        static MANIFEST: OnceLock<Manifest> = OnceLock::new();
        MANIFEST.get_or_init(|| {
            Manifest::from_json(MANIFEST_JSON).expect("crates/ocr/models.json parses")
        })
    }

    /// A manifest from text, so tests can describe files that are not the committed models.
    pub fn from_json(text: &str) -> Result<Manifest, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// The entry every model shares.
    pub fn pinned(&self, model: Model) -> &Pinned {
        match model {
            Model::Det => &self.models.det.pinned,
            Model::Textline => &self.models.textline.pinned,
            Model::Rec => &self.models.rec.pinned,
        }
    }
}

/// Why the models could not be opened or run. Every variant names the file or model it is about,
/// because "a model failed" sends someone to look at three.
#[derive(Debug)]
pub enum ModelError {
    /// Nothing at the manifest's path.
    Missing { what: String, path: PathBuf },
    /// Something is there and could not be read.
    Unreadable {
        what: String,
        path: PathBuf,
        error: io::Error,
    },
    /// The file is not the one the manifest pins.
    Mismatch {
        what: String,
        path: PathBuf,
        expected: String,
        actual: String,
    },
    /// The dictionary's line count is not the manifest's.
    Dictionary {
        what: String,
        path: PathBuf,
        expected: usize,
        actual: usize,
    },
    /// The graph does not have exactly one input and one output.
    Arity {
        what: String,
        inputs: usize,
        outputs: usize,
    },
    /// The graph's input or output is not what the manifest says. Boxed because two specs inline
    /// made every `Result` here 136 bytes wide, for the rarest refusal (clippy's
    /// `result_large_err`).
    Graph {
        what: String,
        side: &'static str,
        expected: Box<TensorSpec>,
        actual: Box<TensorSpec>,
    },
    /// ONNX Runtime refused the bytes, or a run.
    Runtime { what: String, error: ort::Error },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::Missing { what, path } => write!(
                f,
                "{what}: {} is missing. The models are converted once and kept out of git; \
                 crates/ocr/models.json's 'conversion' says how",
                path.display()
            ),
            ModelError::Unreadable { what, path, error } => {
                write!(f, "{what}: {} could not be read: {error}", path.display())
            }
            ModelError::Mismatch {
                what,
                path,
                expected,
                actual,
            } => write!(
                f,
                "{what}: {} is not the file crates/ocr/models.json pins \
                 (sha256 {actual}, expected {expected})",
                path.display()
            ),
            ModelError::Dictionary {
                what,
                path,
                expected,
                actual,
            } => write!(
                f,
                "{what}: {} has {actual} entries, and crates/ocr/models.json says {expected}",
                path.display()
            ),
            ModelError::Arity {
                what,
                inputs,
                outputs,
            } => write!(
                f,
                "{what}: the graph has {inputs} inputs and {outputs} outputs, and the reader \
                 expects one of each"
            ),
            ModelError::Graph {
                what,
                side,
                expected,
                actual,
            } => write!(
                f,
                "{what}: the graph's {side} is {actual}, and crates/ocr/models.json says {expected}"
            ),
            ModelError::Runtime { what, error } => write!(f, "{what}: ONNX Runtime: {error}"),
        }
    }
}

impl std::error::Error for ModelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ModelError::Unreadable { error, .. } => Some(error),
            ModelError::Runtime { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// A dense `f32` tensor, row-major: what goes into a model and what comes out of one. Every model
/// here takes one NCHW image batch and returns one tensor, so this is the whole interface.
#[derive(Clone, Debug, PartialEq)]
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

struct Loaded {
    session: Session,
    what: String,
    input: String,
}

/// The three sessions and the recognition dictionary, verified.
pub struct Models {
    det: Loaded,
    textline: Loaded,
    rec: Loaded,
    characters: Vec<String>,
}

impl Models {
    /// Opens the committed manifest's models from `dir` (normally [`DEFAULT_DIR`] under the repo
    /// root).
    pub fn open(dir: &Path) -> Result<Models, ModelError> {
        Models::open_with(Manifest::committed(), dir)
    }

    /// Opens `manifest`'s models from `dir`: each file read, hashed, refused unless it is the one
    /// pinned, loaded from those same bytes, and its graph checked against the manifest.
    pub fn open_with(manifest: &Manifest, dir: &Path) -> Result<Models, ModelError> {
        let det = load(manifest.pinned(Model::Det), dir)?;
        let textline = load(manifest.pinned(Model::Textline), dir)?;
        let rec = load(manifest.pinned(Model::Rec), dir)?;
        let characters = dictionary(&manifest.models.rec, dir)?;
        Ok(Models {
            det,
            textline,
            rec,
            characters,
        })
    }

    /// The recognition dictionary, in class order from index 1 (index 0 is CTC's blank, and the
    /// last class is the space `use_space_char` appends).
    pub fn characters(&self) -> &[String] {
        &self.characters
    }

    /// Runs `model` once on `input` and returns its one output.
    pub fn run(&mut self, model: Model, input: Tensor) -> Result<Tensor, ModelError> {
        let loaded = match model {
            Model::Det => &mut self.det,
            Model::Textline => &mut self.textline,
            Model::Rec => &mut self.rec,
        };
        let what = &loaded.what;
        let runtime = |error| ModelError::Runtime {
            what: what.clone(),
            error,
        };
        let value = ort::value::Tensor::from_array((input.shape, input.data)).map_err(runtime)?;
        let outputs = loaded
            .session
            .run(ort::inputs![loaded.input.as_str() => value])
            .map_err(runtime)?;
        let (shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(runtime)?;
        Ok(Tensor {
            // An output's dimensions are concrete, never the -1 a graph declaration may carry.
            shape: shape.iter().map(|&d| d as usize).collect(),
            data: data.to_vec(),
        })
    }
}

/// Reads one pinned file and refuses it unless its digest is `sha256`.
fn read_verified(what: &str, dir: &Path, file: &str, sha256: &str) -> Result<Vec<u8>, ModelError> {
    let path = dir.join(file);
    if !path.exists() {
        return Err(ModelError::Missing {
            what: what.to_owned(),
            path,
        });
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(ModelError::Unreadable {
                what: what.to_owned(),
                path,
                error,
            })
        }
    };
    let actual = hex(&Sha256::digest(&bytes));
    if actual != sha256 {
        return Err(ModelError::Mismatch {
            what: what.to_owned(),
            path,
            expected: sha256.to_owned(),
            actual,
        });
    }
    Ok(bytes)
}

fn load(pinned: &Pinned, dir: &Path) -> Result<Loaded, ModelError> {
    let what = pinned.name.clone();
    let bytes = read_verified(&what, dir, &pinned.file, &pinned.sha256)?;
    let runtime = |error| ModelError::Runtime {
        what: what.clone(),
        error,
    };
    let session = Session::builder()
        .and_then(|builder| builder.commit_from_memory(&bytes))
        .map_err(runtime)?;
    // One input and one output each; a second of either is a different graph from the one
    // converted, and the manifest has nowhere to describe it.
    let (inputs, outputs) = (session.inputs.len(), session.outputs.len());
    if inputs != 1 || outputs != 1 {
        return Err(ModelError::Arity {
            what,
            inputs,
            outputs,
        });
    }
    let input = &session.inputs[0];
    check_graph(
        &what,
        "input",
        &pinned.input,
        &input.name,
        &input.input_type,
    )?;
    let output = &session.outputs[0];
    check_graph(
        &what,
        "output",
        &pinned.output,
        &output.name,
        &output.output_type,
    )?;
    Ok(Loaded {
        session,
        what,
        input: pinned.input.name.clone(),
    })
}

fn check_graph(
    what: &str,
    side: &'static str,
    expected: &TensorSpec,
    name: &str,
    value_type: &ort::value::ValueType,
) -> Result<(), ModelError> {
    let actual = TensorSpec {
        name: name.to_owned(),
        // `None` for a sequence or a map, which no model here has; an empty shape then fails the
        // comparison, which is the right answer.
        shape: value_type
            .tensor_shape()
            .map(|shape| shape.to_vec())
            .unwrap_or_default(),
    };
    if &actual == expected {
        Ok(())
    } else {
        Err(ModelError::Graph {
            what: what.to_owned(),
            side,
            expected: Box::new(expected.clone()),
            actual: Box::new(actual),
        })
    }
}

fn dictionary(rec: &Rec, dir: &Path) -> Result<Vec<String>, ModelError> {
    let what = format!("{}'s dictionary", rec.pinned.name);
    let bytes = read_verified(&what, dir, &rec.dict.file, &rec.dict.sha256)?;
    let path = dir.join(&rec.dict.file);
    // The digest has already said these are the bytes that were written, and they were written as
    // UTF-8; a decoding failure here would mean the manifest pins a file it never described.
    let text = String::from_utf8(bytes).map_err(|error| ModelError::Unreadable {
        what: what.clone(),
        path: path.clone(),
        error: io::Error::new(io::ErrorKind::InvalidData, error),
    })?;
    // One entry per line, LF, with a final newline. `split('\n')` rather than `lines()`, because
    // `lines()` also strips a `\r`, and an entry is exactly what the file holds between newlines.
    let body = text.strip_suffix('\n').unwrap_or(&text);
    let characters: Vec<String> = body.split('\n').map(str::to_owned).collect();
    if characters.len() != rec.dict.entries {
        return Err(ModelError::Dictionary {
            what,
            path,
            expected: rec.dict.entries,
            actual: characters.len(),
        });
    }
    Ok(characters)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
