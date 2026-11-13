//! `golf-lab read-tape`, run as the binary on photos this test draws. [M46 P8]
//!
//! The done-when of the M46 plan's P8: two synthetic PNGs give two JSON lines and their review
//! files, and an unknown flag and a photo that is not there are each refused with their sentences.
//! Every photo here is `no_label`. That was the stub's answer until P9, and since P9 it is the label
//! finder's, because a plain pale rectangle this small is no STRIKEPRO label: it samples under
//! `impact::tape::MIN_PX_PER_MM`, and it has neither the outline nor the logo. What the verb prints
//! and draws for a found label, a reading and a pick is held by `golf_lab::read_tape`'s own tests,
//! over outcomes built by hand, and the finder's own tests are `crates/impact/tests/tape_label.rs`. No real photo is read here, since
//! none of a struck label exists before P29 (the plan's P3 finding 10).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use image::{Rgb, RgbImage};
use serde_json::Value;

fn golf_lab(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_golf-lab"))
        .args(args)
        .output()
        .expect("golf-lab runs")
}

/// A photo of a pale label-sized patch with one dark mark on it, on a darker face, `width` by
/// `height`: enough to look like the thing the verb is for, though the finder finds nothing in it.
fn draw_photo(path: &Path, width: u32, height: u32) {
    let mut image = RgbImage::from_pixel(width, height, Rgb([45, 45, 50]));
    for y in height / 4..height * 3 / 4 {
        for x in width / 5..width * 4 / 5 {
            image.put_pixel(x, y, Rgb([235, 235, 228]));
        }
    }
    for y in height / 2 - 6..height / 2 + 6 {
        for x in width / 2 - 6..width / 2 + 6 {
            image.put_pixel(x, y, Rgb([120, 120, 125]));
        }
    }
    image.save(path).unwrap();
}

fn lines(out: &Output) -> Vec<Value> {
    String::from_utf8(out.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is one JSON object"))
        .collect()
}

#[test]
fn two_photos_give_two_lines_and_their_review_images() {
    let dir = tempfile::tempdir().unwrap();
    let (first, second) = (
        dir.path().join("IMG_0001.png"),
        dir.path().join("IMG_0002.png"),
    );
    draw_photo(&first, 320, 200);
    draw_photo(&second, 300, 240);
    let review = dir.path().join("review");
    let out = golf_lab(&[
        "read-tape",
        &first.to_string_lossy(),
        &second.to_string_lossy(),
        "--label",
        "strikepro_iron",
        "--sequence",
        "--review",
        &review.to_string_lossy(),
    ]);
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(1), "no photo was read, by the stub");
    let lines = lines(&out);
    assert_eq!(lines.len(), 2);
    for (line, name) in lines.iter().zip(["IMG_0001.png", "IMG_0002.png"]) {
        assert_eq!(line["photo"], name);
        assert_eq!(line["label"], "strikepro_iron");
        assert_eq!(line["outcome"], "not_read");
        assert_eq!(line["reason"], "no_label");
        assert_eq!(
            line["sentence"],
            contracts::drill::TapeNotRead::NoLabel.sentence()
        );
        assert_eq!(line["retake_helps"], true);
        assert_eq!(
            line["previous"],
            Value::Null,
            "nothing was placed to diff against"
        );
        assert!(line.get("error").is_none());
    }
    for (stem, size) in [("IMG_0001", (320, 200)), ("IMG_0002", (300, 240))] {
        let photo = image::open(review.join(stem).join("photo.png"))
            .unwrap()
            .to_rgb8();
        assert_eq!(photo.dimensions(), size, "{stem}");
        assert_eq!(
            photo.get_pixel(size.0 / 2, size.1 / 2),
            &Rgb([120, 120, 125])
        );
        assert!(
            !review.join(stem).join("crop.png").exists(),
            "{stem}: no label, no crop"
        );
    }
}

#[test]
fn a_photo_that_will_not_decode_is_its_own_line() {
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("IMG_0003.jpg");
    fs::write(&photo, b"not a photo").unwrap();
    let out = golf_lab(&[
        "read-tape",
        "--label=strikepro_driver",
        &photo.to_string_lossy(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    let lines = lines(&out);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["reason"], "decode_failed");
    assert_eq!(lines[0]["label"], "strikepro_driver");
    let error = lines[0]["error"].as_str().unwrap();
    assert!(error.starts_with("could not read image "), "{error}");
}

#[test]
fn help_is_the_usage_and_every_refusal_names_what_was_wrong() {
    let out = golf_lab(&["read-tape", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("golf-lab read-tape PHOTO... --label T [--sequence] [--review DIR]"));

    for (args, problem) in [
        (
            &["read-tape"][..],
            "read-tape: the following arguments are required: PHOTO, --label",
        ),
        (
            &["read-tape", "a.jpg"][..],
            "read-tape: the following arguments are required: --label",
        ),
        (
            &["read-tape", "--label", "strikepro_iron"][..],
            "read-tape: the following arguments are required: PHOTO",
        ),
        (
            &["read-tape", "a.jpg", "--label", "iron"][..],
            "read-tape: argument --label: invalid choice: 'iron' (choose from \
             'strikepro_driver', 'strikepro_iron', 'strikepro_hybrid')",
        ),
        (
            &["read-tape", "a.jpg", "--label"][..],
            "read-tape: argument --label: expected one argument",
        ),
        (
            &[
                "read-tape",
                "a.jpg",
                "--label",
                "strikepro_iron",
                "--review",
            ][..],
            "read-tape: argument --review: expected one argument",
        ),
        (
            &[
                "read-tape",
                "a.jpg",
                "--label",
                "strikepro_iron",
                "--sequence=yes",
            ][..],
            "read-tape: argument --sequence: ignored explicit argument \"yes\"",
        ),
        (
            &["read-tape", "a.jpg", "--label", "strikepro_iron", "--json"][..],
            "read-tape: unrecognized arguments: --json",
        ),
    ] {
        let out = golf_lab(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("golf-lab: {problem}\n")),
            "{args:?}: {stderr}"
        );
        assert!(stderr.contains("usage: golf-lab"), "{args:?}");
    }
}

#[test]
fn a_photo_that_is_not_there_is_refused_before_any_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let there = dir.path().join("IMG_0001.png");
    draw_photo(&there, 64, 48);
    let absent = dir.path().join("IMG_0002.png");
    let review = dir.path().join("review");
    let out = golf_lab(&[
        "read-tape",
        &there.to_string_lossy(),
        &absent.to_string_lossy(),
        "--label",
        "strikepro_hybrid",
        "--review",
        &review.to_string_lossy(),
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        out.stdout.is_empty(),
        "no line, not even the photo that is there"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.starts_with("golf-lab read-tape: "), "{stderr}");
    assert!(
        stderr.contains("IMG_0002.png not found, so no photo was read"),
        "{stderr}"
    );
    assert!(!review.exists(), "nothing was reviewed");
}
