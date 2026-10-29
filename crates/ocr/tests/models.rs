//! The model manifest, and every way `Models::open` refuses (M29 P1).
//!
//! The converted models are gitignored, so the one test that runs them is ignored and says what it
//! needs; it is run by `cargo test -p ocr -- --include-ignored` on the box that holds them, and it
//! fails rather than passes when they are absent. Every refusal is tested without them, on files
//! a test writes.

use std::fs;
use std::path::{Path, PathBuf};

use ocr::models::{Manifest, Model, ModelError, Models, Tensor, DEFAULT_DIR, MANIFEST_JSON};
use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// `Models` holds sessions and is not `Debug`, so `unwrap_err` is not available.
fn refusal(opened: Result<Models, ModelError>) -> ModelError {
    match opened {
        Ok(_) => panic!("opened, and should have been refused"),
        Err(error) => error,
    }
}

#[test]
fn the_committed_manifest_pins_every_file_it_names() {
    let manifest = Manifest::committed();
    for model in Model::ALL {
        let pinned = manifest.pinned(model);
        assert!(
            is_sha256(&pinned.sha256),
            "{}: sha256 {:?}",
            model.key(),
            pinned.sha256
        );
        assert!(
            is_sha256(&pinned.source.pdiparams_sha256),
            "{}: source digest",
            model.key()
        );
        // A model's directory is its Paddle name, so a file can never be pinned under another
        // model's name.
        assert_eq!(
            pinned.file,
            format!("{}/model.onnx", pinned.name),
            "{}",
            model.key()
        );
        assert!(
            pinned.source.dir.ends_with(&pinned.name),
            "{}: source dir",
            model.key()
        );
    }
    let rec = &manifest.models.rec;
    assert!(is_sha256(&rec.dict.sha256));
    assert_eq!(rec.dict.file, format!("{}/dict.txt", rec.pinned.name));
}

#[test]
fn recognition_has_a_class_for_every_dictionary_entry_plus_blank_and_space() {
    // PaddleX's CTCLabelDecode puts `blank` before the dictionary and `use_space_char`'s space
    // after it. Checked here because it is the one cross-check that needs no model: the manifest's
    // graph shape, which `open` holds the session to, against its own dictionary count.
    let rec = &Manifest::committed().models.rec;
    assert_eq!(
        rec.pinned.output.shape.last(),
        Some(&(rec.dict.entries as i64 + 2))
    );
}

#[test]
fn every_fixed_dimension_agrees_with_the_constants_beside_it() {
    let models = &Manifest::committed().models;
    let textline = &models.textline;
    let (width, height) = (textline.resize.width as i64, textline.resize.height as i64);
    assert_eq!(textline.pinned.input.shape[1..], [3, height, width]);
    assert_eq!(
        textline.pinned.output.shape[1],
        textline.labels.len() as i64
    );
    let rec = &models.rec;
    assert_eq!(
        rec.pinned.input.shape[1..3],
        [rec.image_shape[0] as i64, rec.image_shape[1] as i64]
    );
    assert_eq!(models.det.pinned.input.shape[1], 3);
}

#[test]
fn an_empty_directory_is_refused_naming_the_first_model_and_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let error = refusal(Models::open(dir.path()));
    match &error {
        ModelError::Missing { what, path } => {
            assert_eq!(what, "PP-OCRv6_medium_det");
            assert!(
                path.ends_with("PP-OCRv6_medium_det/model.onnx"),
                "{}",
                path.display()
            );
        }
        other => panic!("expected Missing, got {other}"),
    }
    let message = error.to_string();
    assert!(
        message.contains("PP-OCRv6_medium_det") && message.contains("is missing"),
        "{message}"
    );
}

#[test]
fn a_file_that_is_not_the_pinned_one_is_refused_with_both_digests() {
    let dir = tempfile::tempdir().unwrap();
    let pinned = Manifest::committed().pinned(Model::Det);
    let path = dir.path().join(&pinned.file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not a model").unwrap();
    match refusal(Models::open(dir.path())) {
        ModelError::Mismatch {
            what,
            expected,
            actual,
            ..
        } => {
            assert_eq!(what, "PP-OCRv6_medium_det");
            assert_eq!(expected, pinned.sha256);
            assert_eq!(actual, hex_digest(b"not a model"));
        }
        other => panic!("expected Mismatch, got {other}"),
    }
}

#[test]
fn something_unreadable_at_the_path_is_refused_as_unreadable_not_missing() {
    let dir = tempfile::tempdir().unwrap();
    let pinned = Manifest::committed().pinned(Model::Det);
    // A directory where the file should be: it exists, and reading it fails.
    fs::create_dir_all(dir.path().join(&pinned.file)).unwrap();
    match refusal(Models::open(dir.path())) {
        ModelError::Unreadable { what, .. } => assert_eq!(what, "PP-OCRv6_medium_det"),
        other => panic!("expected Unreadable, got {other}"),
    }
}

#[test]
fn bytes_matching_their_digest_that_are_not_a_model_are_refused_by_the_runtime() {
    // The digest proves the file is the one pinned, not that it is a graph; a manifest pinning the
    // wrong file is caught by ONNX Runtime, and still named.
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"pinned, and still not a model";
    let mut manifest: serde_json::Value = serde_json::from_str(MANIFEST_JSON).unwrap();
    manifest["models"]["det"]["sha256"] = hex_digest(bytes).into();
    let manifest = Manifest::from_json(&manifest.to_string()).unwrap();
    let path = dir.path().join(&manifest.pinned(Model::Det).file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    match refusal(Models::open_with(&manifest, dir.path())) {
        ModelError::Runtime { what, .. } => assert_eq!(what, "PP-OCRv6_medium_det"),
        other => panic!("expected Runtime, got {other}"),
    }
}

#[test]
#[ignore = "needs the converted models in data/models/ocr/ (gitignored; crates/ocr/models.json's 'conversion' says how they were made)"]
fn every_model_loads_and_runs_once_on_zeros() {
    let mut models =
        Models::open(&repo_root().join(DEFAULT_DIR)).unwrap_or_else(|error| panic!("{error}"));
    let manifest = Manifest::committed();
    assert_eq!(models.characters().len(), manifest.models.rec.dict.entries);
    assert_eq!(models.characters()[0], "!");

    // Detection takes any height and width that are multiples of 32, and answers one probability
    // per pixel.
    let det = models.run(Model::Det, zeros(&[1, 3, 96, 128])).unwrap();
    assert_eq!(det.shape, [1, 1, 96, 128]);
    assert!(
        det.data.iter().all(|p| (0.0..=1.0).contains(p)),
        "det probabilities"
    );

    // Orientation answers a softmax over its two labels.
    let textline = models
        .run(Model::Textline, zeros(&[1, 3, 80, 160]))
        .unwrap();
    assert_eq!(textline.shape, [1, 2]);
    assert!(
        (textline.data.iter().sum::<f32>() - 1.0).abs() < 1e-5,
        "{:?}",
        textline.data
    );

    // Recognition answers one distribution per 8 pixels of width, over the dictionary's classes.
    let rec = models.run(Model::Rec, zeros(&[1, 3, 48, 320])).unwrap();
    assert_eq!(rec.shape, [1, 40, models.characters().len() + 2]);
    let first: f32 = rec.data[..rec.shape[2]].iter().sum();
    assert!((first - 1.0).abs() < 1e-4, "rec step 0 sums to {first}");
}

fn zeros(shape: &[usize]) -> Tensor {
    Tensor {
        shape: shape.to_vec(),
        data: vec![0.0; shape.iter().product()],
    }
}
