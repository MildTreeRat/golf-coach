//! `golf-ocr`'s refusals, which need neither the models nor the bay photos (M29 P7).
//!
//! What it reads off a photo is `tests/read.rs`'s gate, through the library the binary calls; this
//! holds the command line to its exit codes, and each refusal to its cause's own message.

use std::process::{Command, Output};

use image::RgbImage;

fn golf_ocr(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_golf-ocr"))
        .args(args)
        .output()
        .expect("golf-ocr runs")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_malformed_command_line_exits_two_with_the_usage() {
    for args in [
        &["parse"][..],
        &["read"][..],
        &["read", "a.png", "b.png"][..],
        &["read", "a.png", "--device"][..],
        &["read", "a.png", "--min-confidence", "high"][..],
        &["read", "a.png", "--verbose"][..],
    ] {
        let output = golf_ocr(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(
            stderr(&output).contains("usage: golf-ocr read <photo>"),
            "{args:?}"
        );
        assert!(output.stdout.is_empty(), "{args:?}");
    }
    let help = golf_ocr(&[]);
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).starts_with("usage: golf-ocr read"));
}

/// Each refusal exits 1 with nothing on stdout, and says what was wrong in its cause's words: the
/// device no profile names, the photo that is not there, the model that is missing.
#[test]
fn a_refused_read_exits_one_naming_its_cause() {
    let scratch = tempfile::tempdir().unwrap();
    let photo = scratch.path().join("screen.png");
    RgbImage::new(8, 8).save(&photo).unwrap();
    let photo = photo.to_str().unwrap();
    let empty = scratch.path().join("no-models");
    std::fs::create_dir(&empty).unwrap();
    let missing = scratch.path().join("missing.png");

    let cases = [
        (
            vec!["read", photo, "--device", "no_such_device"],
            "no_such_device",
        ),
        (
            vec!["read", missing.to_str().unwrap()],
            "could not read image",
        ),
        (
            vec!["read", photo, "--models", empty.to_str().unwrap()],
            "is missing. The models are converted once",
        ),
    ];
    for (args, says) in cases {
        let output = golf_ocr(&args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty(), "{args:?}");
        let message = stderr(&output);
        assert!(
            message.starts_with("golf-ocr read: "),
            "{args:?}: {message}"
        );
        assert!(message.contains(says), "{args:?}: {message}");
    }
}
