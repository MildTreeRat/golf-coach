//! `golf-lab import-video` against a fake decoder and a reader replaying a committed screen vector:
//! a Live Photo's clip imported on its own, the M42 plan's decision 7. [M42 P14b]
//!
//! **No Python twin**: no script ever read a clip. The reader answers with
//! `spec/vectors/screen/hand/full-swing-grid.json`'s boxes, so the parse, the validation and the
//! stamp are `crates/screen`'s real ones. The decoder is a fake: a "clip" is stand-in bytes it never
//! opens, whose grid frames are a few small rasters seeded by the clip's name, and whose replay is
//! whatever the case asks for, a strike drawn here that the real `impact::read_impact` reads, or a
//! blank field it refuses. `tests/full_swing_clips.rs` runs the real decode and the real reader on
//! the five grid clips. These run in `cargo test` on any machine.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::unscored::UnscoredReason;
use contracts::Timestamp;
use golf_lab::import::{import_photos, import_video, IntoSession, Request};
use golf_lab::read_impact::DECODE_WIDTH;
use golf_lab::shot::{encode_png, MotionFrames, PhotoReader, Prepared, ShotOptions, GRID_FRAME};
use impact::{read_impact, Frame, BALL_DIAMETER_MM};
use screen::profile::DeviceProfile;
use screen::{tile_refusals, ScreenInput, IMPACT_VIDEO_MISREAD};
use serde_json::Value;
use storage::bag_store::BagStore;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::{read_corpus, EngineVersions};
use storage::manifest::Role;
use storage::shot_store::{hash_image, ShotStore};
use storage::state::load_shot_analysis;

const SESSION: &str = "2026-10-23";
const CLIP: &str = "IMG_0001.MOV";
const FIELD: &str = "impact_offset_h";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> Value {
    let path = repo().join(relative);
    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
}

fn now() -> Timestamp {
    "2026-10-23T08:00:00Z".parse().unwrap()
}

/// A reader answering with `IMG_2889`'s hand-worked boxes, recording each file it was asked to read.
struct Replay {
    prepared: Prepared,
    asked: Vec<PathBuf>,
}

impl Replay {
    fn grid() -> Self {
        let document = read_json("spec/vectors/screen/hand/full-swing-grid.json");
        let input: ScreenInput = serde_json::from_value(document["input"].clone()).unwrap();
        Replay {
            prepared: Prepared {
                boxes: input.boxes,
                notes: input.notes,
            },
            asked: Vec::new(),
        }
    }

    /// No box at all, so the parse is empty: a read of nothing.
    fn blank() -> Self {
        Replay {
            prepared: Prepared {
                boxes: Vec::new(),
                notes: Vec::new(),
            },
            asked: Vec::new(),
        }
    }
}

impl PhotoReader for Replay {
    fn ready(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn prepare(&mut self, photo: &Path, profile: &DeviceProfile) -> Result<Prepared, String> {
        assert_eq!(profile.device, "full_swing");
        self.asked.push(photo.to_path_buf());
        Ok(self.prepared.clone())
    }
}

/// A clip's grid frames: three small rasters seeded by its name, so each clip has its own photo and
/// the same clip always has the same one.
fn grid_frames(name: &str) -> Vec<Frame> {
    let seed = name.bytes().fold(7u8, u8::wrapping_add);
    (0..3u8)
        .map(|index| {
            let rgb = (0..48u8)
                .flat_map(|pixel| {
                    let shade = seed.wrapping_add(pixel.wrapping_mul(index + 1));
                    [shade, shade, shade]
                })
                .collect();
            Frame {
                width: 8,
                height: 6,
                rgb,
            }
        })
        .collect()
}

/// The photo [`grid_frames`] gives, as `write_grid_frame` writes it.
fn grid_png(name: &str) -> Vec<u8> {
    let frames = grid_frames(name);
    encode_png(&frames[GRID_FRAME.choose(&frames).unwrap()]).unwrap()
}

/// The fake decoder: the grid at a clip's own size, the replay at the reader's. A clip named in
/// `undecodable` fails at its own size, as a file ffmpeg cannot open does.
struct Clips {
    replay: Result<Vec<Frame>, String>,
    undecodable: Vec<&'static str>,
    asked: Vec<(String, Option<u32>)>,
}

impl Clips {
    fn answering(replay: Result<Vec<Frame>, String>) -> Self {
        Clips {
            replay,
            undecodable: Vec::new(),
            asked: Vec::new(),
        }
    }

    /// A replay the reader reads, the offset being what [`offset_of_strike`] says.
    fn strike() -> Self {
        Self::answering(Ok(strike()))
    }

    /// A replay holding nothing the reader can find: `NoBall`.
    fn blank() -> Self {
        let blank = Frame {
            width: 64,
            height: 48,
            rgb: vec![40; 64 * 48 * 3],
        };
        Self::answering(Ok(vec![blank; 5]))
    }
}

impl MotionFrames for Clips {
    fn frames(&mut self, clip: &Path, width: Option<u32>) -> Result<Vec<Frame>, String> {
        let name = clip.file_name().unwrap().to_string_lossy().into_owned();
        self.asked.push((name.clone(), width));
        match width {
            None if self.undecodable.contains(&name.as_str()) => {
                Err(format!("ffmpeg could not decode the video of {name}: boom"))
            }
            None => Ok(grid_frames(&name)),
            Some(_) => self.replay.clone(),
        }
    }
}

const STRIKE_WIDTH: usize = 960;
const STRIKE_HEIGHT: usize = 720;
const STRIKE_BALL: [f64; 2] = [480.0, 360.0];
/// A ball this many pixels across scales the strike, as `crates/impact/tests/impact.rs` draws it.
const STRIKE_D: f64 = 24.0;

/// Paints every pixel of `frame` whose centre `inside` accepts, by its offset from `at`, within
/// `reach` of it on both axes.
fn paint(
    frame: &mut Frame,
    at: [f64; 2],
    reach: f64,
    rgb: [u8; 3],
    inside: impl Fn(f64, f64) -> bool,
) {
    let span = |c: f64, limit: usize| {
        let low = (c - reach).floor().max(0.0) as usize;
        let high = ((c + reach).ceil().max(0.0) as usize).min(limit);
        low..high
    };
    for y in span(at[1], frame.height) {
        for x in span(at[0], frame.width) {
            if inside(x as f64 + 0.5 - at[0], y as f64 + 0.5 - at[1]) {
                let pixel = (y * frame.width + x) * 3;
                frame.rgb[pixel..pixel + 3].copy_from_slice(&rgb);
            }
        }
    }
}

/// `crates/impact/tests/impact.rs`'s strike, the one the reader's own gate draws, with the ball
/// 10 mm toward the toe of the head's midpoint: the ball at rest to frame 5 and gone after, and a
/// light head with a dark shaft at its heel, reaching the ball in frame 6. Here it is only something
/// the real reader reads, so which number it reads is the reader's, by [`offset_of_strike`].
fn strike() -> Vec<Frame> {
    let angle = 30f64.to_radians();
    let toe = [angle.cos(), angle.sin()];
    let forward = [-toe[1], toe[0]];
    let moved = |from: [f64; 2], along: f64, across: f64| {
        [
            from[0] + along * toe[0] + across * forward[0],
            from[1] + along * toe[1] + across * forward[1],
        ]
    };
    let bar = |frame: &mut Frame, at: [f64; 2], length: f64, width: f64, rgb: [u8; 3]| {
        let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
        paint(frame, at, (length + width) / 2.0 + 1.0, rgb, |dx, dy| {
            dot([dx, dy], toe).abs() <= length / 2.0 && dot([dx, dy], forward).abs() <= width / 2.0
        });
    };
    let contact = moved(STRIKE_BALL, -10.0 * STRIKE_D / BALL_DIAMETER_MM, 0.0);
    (0..9)
        .map(|index| {
            let mut frame = Frame {
                width: STRIKE_WIDTH,
                height: STRIKE_HEIGHT,
                rgb: [60, 80, 50].repeat(STRIKE_WIDTH * STRIKE_HEIGHT),
            };
            if index <= 5 {
                let radius = STRIKE_D / 2.0;
                paint(
                    &mut frame,
                    STRIKE_BALL,
                    radius + 1.0,
                    [255, 255, 255],
                    |dx, dy| dx * dx + dy * dy <= radius * radius,
                );
            }
            if index >= 2 {
                let centre = moved(contact, 0.0, (f64::from(index) - 6.0) * STRIKE_D);
                let hosel = moved(centre, -90.0, 0.0);
                bar(&mut frame, hosel, 120.0, 6.0, [20, 20, 20]);
                bar(&mut frame, centre, 60.0, 10.0, [230, 230, 230]);
            }
            frame
        })
        .collect()
}

/// What the real reader reads off [`strike`].
fn offset_of_strike() -> f64 {
    let reading = read_impact(&strike()).unwrap_or_else(|refusal| panic!("{}", refusal.sentence()));
    assert!(reading.offset_h_mm > 0.0, "{}", reading.offset_h_mm);
    reading.offset_h_mm
}

/// A scratch root: `clips/` with the stand-ins, `golfers/` holding `aaron` and the bag the storage
/// family records for him, as `tests/import_photos.rs` makes it. The clip's time is a day old, so a
/// photo that kept it did not get it by accident.
struct Scratch {
    root: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let golfers = root.path().join("golfers");
        fs::create_dir_all(root.path().join("clips")).unwrap();
        fs::create_dir_all(&golfers).unwrap();
        fs::write(
            golfers.join("aaron.golfer.json"),
            r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "right",
                "created_at": "2026-08-07T15:04:05.123456Z"}"#,
        )
        .unwrap();
        let bags = read_json("spec/vectors/storage/stores/bag-reads.json");
        fs::write(
            BagStore::new(&golfers).path_for("aaron"),
            bags["input"]["files"]["aaron.bag.json"].as_str().unwrap(),
        )
        .unwrap();
        let scratch = Scratch { root };
        scratch.clip(CLIP);
        scratch
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.root.path().to_path_buf(), |path, part| path.join(part))
    }

    /// A stand-in clip under `clips/`, with its old time.
    fn clip(&self, name: &str) -> PathBuf {
        let path = self.path("clips").join(name);
        fs::write(&path, format!("the bytes of {name}, no decoder opens")).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(86_400))
            .unwrap();
        path
    }

    fn store(&self) -> ShotStore {
        ShotStore::new(self.path("shots"))
    }

    fn sessions(&self) -> SwingBundleStore {
        SwingBundleStore::new(self.path("sessions"))
    }

    fn request(&self, paths: &[PathBuf]) -> Request {
        let mut options = ShotOptions::new(self.path("shots"));
        options.device = "full_swing".to_string();
        Request {
            paths: paths.to_vec(),
            session: Some("bay".to_string()),
            options,
            into: None,
        }
    }

    fn entries_request(&self, paths: &[PathBuf]) -> Request {
        let mut request = self.request(paths);
        request.session = None;
        request.into = Some(IntoSession {
            session_id: SESSION.to_string(),
            player: "Aaron".to_string(),
            club: "7 iron".to_string(),
            sessions_dir: self.path("sessions"),
            golfers_dir: self.path("golfers"),
        });
        request
    }
}

/// One `import-video`: the exit code, stdout and stderr.
fn run(request: &Request, motion: &mut Clips, reader: &mut Replay) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = import_video(request, motion, reader, now(), &mut out, &mut err).unwrap();
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn listed(shot: &contracts::shot::ShotData) -> bool {
    let provenance = shot.provenance.as_ref().unwrap();
    let present = provenance.fields_present.as_ref().unwrap();
    present.iter().any(|field| field == FIELD)
}

/// A replay the reader reads sets the offset and lists it, and the shot is whole: no warning, no
/// flag, and the row and the line under it say so. The store-only shot's photo is gone with the run,
/// so it names the clip it came from, and its time is the clip's.
#[test]
fn a_replay_read_sets_the_offset_and_lists_it() {
    let scratch = Scratch::new();
    let clip = scratch.path("clips").join(CLIP);
    let request = scratch.request(std::slice::from_ref(&clip));
    let mut motion = Clips::strike();
    let (code, out, err) = run(&request, &mut motion, &mut Replay::grid());
    assert_eq!(err, "");
    assert_eq!(code, 0, "{out}");
    let offset = offset_of_strike();
    assert!(
        out.starts_with("\nImporting 1 clip(s) as session 'bay' [full_swing]\n"),
        "{out}"
    );
    assert!(
        out.contains(&format!(
            "\n      impact video: offset {offset:+.1} mm (toe)\n"
        )),
        "{out}"
    );
    assert!(out.ends_with("\n1 shot(s) imported cleanly.\n"), "{out}");
    assert_eq!(
        motion.asked,
        [
            (CLIP.to_string(), None),
            (CLIP.to_string(), Some(DECODE_WIDTH))
        ]
    );

    let digest = hash_image(&grid_png(CLIP));
    let shot = scratch.store().get(&digest).unwrap().unwrap();
    assert_eq!(shot.impact_offset_h, Some(offset));
    assert!(listed(&shot));
    assert_eq!(shot.carry_distance, Some(131.0), "the grid's numbers too");
    assert_eq!(shot.shot_id, format!("bay-{}", &digest[..12]));
    assert_eq!(shot.timestamp, ocr::screen_timestamp(&clip).unwrap());
    assert!(tile_refusals(&shot)
        .unwrap()
        .iter()
        .all(|entry| entry.name != FIELD));
    let provenance = shot.provenance.as_ref().unwrap();
    assert_eq!(provenance.warnings, Vec::<String>::new());
    assert!(!provenance.needs_review);
    assert_eq!(
        provenance.image_path,
        Some(clip.display().to_string()),
        "the scratch PNG is gone, and the clip is still there"
    );
}

/// A replay the reader refuses still stores the shot: the field listed and `None`, the refusal's
/// sentence a warning, `tile_refusals` naming it `misread`, and the numbers not flagged for it.
#[test]
fn a_replay_refused_lists_it_leaves_it_none_warns_and_is_misread() {
    let scratch = Scratch::new();
    let clip = scratch.path("clips").join(CLIP);
    let request = scratch.request(&[clip]);
    let (code, out, err) = run(&request, &mut Clips::blank(), &mut Replay::grid());
    assert_eq!(err, "");
    assert_eq!(code, 0, "the numbers are read: {out}");
    let sentence = "impact video: no ball could be found sitting still in the replay";
    assert!(out.contains(&format!("\n      {sentence}\n")), "{out}");
    assert!(
        out.ends_with("\n1 shot(s) imported cleanly, 1 of them with no impact offset.\n"),
        "{out}"
    );

    let shot = scratch
        .store()
        .get(&hash_image(&grid_png(CLIP)))
        .unwrap()
        .unwrap();
    assert_eq!(shot.impact_offset_h, None);
    assert!(listed(&shot));
    let provenance = shot.provenance.as_ref().unwrap();
    assert_eq!(provenance.warnings, [sentence.to_string()]);
    assert!(!provenance.needs_review);
    let refused = tile_refusals(&shot).unwrap();
    let entry = refused
        .iter()
        .find(|entry| entry.name == FIELD)
        .expect("named");
    assert_eq!(entry.reason, UnscoredReason::Misread);
    assert_eq!(entry.detail, IMPACT_VIDEO_MISREAD);
}

/// A clip that will not decode at its own size fails that clip alone, as an unreadable photo does,
/// and the others still run. A replay that will not decode once the grid has is call 13's third
/// outcome: the shot is stored, with a warning, and the field is not listed.
#[test]
fn an_undecodable_clip_fails_that_clip_alone() {
    let scratch = Scratch::new();
    let bad = scratch.clip("IMG_0000.MOV");
    let good = scratch.path("clips").join(CLIP);
    let request = scratch.request(&[bad, good]);
    let mut motion = Clips::strike();
    motion.undecodable = vec!["IMG_0000.MOV"];
    let (code, out, err) = run(&request, &mut motion, &mut Replay::grid());
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("  IMG_0000.MOV                 failed\n"),
        "{out}"
    );
    assert_eq!(
        err,
        "error: ffmpeg could not decode the video of IMG_0000.MOV: boom\n"
    );
    assert!(
        out.ends_with("\n1 unreadable, 0 flagged for review.\n"),
        "{out}"
    );
    assert_eq!(scratch.store().all().len(), 1, "the other clip is imported");
    assert!(
        !motion
            .asked
            .contains(&("IMG_0000.MOV".to_string(), Some(DECODE_WIDTH))),
        "no grid, so no replay is asked for"
    );

    let scratch = Scratch::new();
    let request = scratch.request(&[scratch.path("clips").join(CLIP)]);
    let mut motion = Clips::answering(Err("boom".to_string()));
    let (code, out, err) = run(&request, &mut motion, &mut Replay::grid());
    assert_eq!(err, "");
    assert_eq!(code, 0, "{out}");
    let warning = "impact video: could not be decoded: boom";
    assert!(out.contains(&format!("\n      {warning}\n")), "{out}");
    let shot = scratch
        .store()
        .get(&hash_image(&grid_png(CLIP)))
        .unwrap()
        .unwrap();
    assert_eq!(shot.impact_offset_h, None);
    assert!(!listed(&shot), "a machine problem: not printed");
    assert_eq!(shot.provenance.as_ref().unwrap().warnings, [warning]);
    assert!(tile_refusals(&shot)
        .unwrap()
        .iter()
        .all(|entry| entry.name != FIELD));
}

/// A device with no field from the replay is refused before anything is read, naming the one that
/// has. A clip nothing is read from (the reader's answer, not the decoder's) is the row's `failed`,
/// and asks for no replay.
#[test]
fn a_device_with_no_impact_video_field_is_refused_and_an_unread_grid_asks_for_no_replay() {
    let scratch = Scratch::new();
    let mut request = scratch.request(&[scratch.path("clips").join(CLIP)]);
    request.options.device = "hd_golf".to_string();
    let mut motion = Clips::strike();
    let mut reader = Replay::grid();
    let (code, out, err) = run(&request, &mut motion, &mut reader);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "error: device 'hd_golf' declares no field read from an impact video, so a clip of it \
         cannot be imported; full_swing does\n"
    );
    assert!(motion.asked.is_empty());
    assert!(reader.asked.is_empty());

    let request = scratch.request(&[scratch.path("clips").join(CLIP)]);
    let mut motion = Clips::strike();
    let (code, out, _) = run(&request, &mut motion, &mut Replay::blank());
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("  IMG_0001.MOV                 failed\n"),
        "{out}"
    );
    assert_eq!(motion.asked, [(CLIP.to_string(), None)]);
    assert!(scratch.store().all().is_empty());
}

/// The two methods each take their own half of a Live Photo, and nothing pairs them (decision 7):
/// `import-photos` over a directory holding both reads the still alone, whose shot never lists the
/// field (it has no frames to ask for: the verb takes no decoder), and `import-video` over it reads
/// the clip alone, so importing both makes two shots. A still named to `import-video` is warned
/// about and skipped.
#[test]
fn each_method_takes_its_own_half_of_a_live_photo() {
    let scratch = Scratch::new();
    let still = scratch.path("clips/IMG_0001.HEIC");
    fs::write(&still, b"a stand-in only the reader would open").unwrap();
    let directory = scratch.path("clips");
    let request = scratch.request(std::slice::from_ref(&directory));

    let mut reader = Replay::grid();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = import_photos(&request, &mut reader, now(), &mut out, &mut err).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("Importing 1 image(s) as session 'bay'"),
        "{out}"
    );
    assert!(!out.contains("impact video"), "{out}");
    assert_eq!(reader.asked, [still.clone()]);
    let digest = hash_image(&fs::read(&still).unwrap());
    let from_still = scratch.store().get(&digest).unwrap().unwrap();
    assert!(!listed(&from_still));
    assert_eq!(from_still.impact_offset_h, None);
    assert!(tile_refusals(&from_still)
        .unwrap()
        .iter()
        .all(|entry| entry.name != FIELD));

    let mut reader = Replay::grid();
    let (code, out, err) = run(&request, &mut Clips::blank(), &mut reader);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("Importing 1 clip(s) as session 'bay'"),
        "{out}"
    );
    let [read] = reader.asked.as_slice() else {
        panic!("{:?}", reader.asked);
    };
    assert_eq!(read.file_name().unwrap(), "IMG_0001.png");
    assert_eq!(scratch.store().all().len(), 2, "two shots of one swing");

    let request = scratch.request(std::slice::from_ref(&still));
    let (code, _, err) = run(&request, &mut Clips::blank(), &mut Replay::grid());
    assert_eq!(code, 1);
    assert_eq!(
        err,
        format!(
            "warning: skipping {} (not a Live Photo clip (.mov): a photo of the screen is read by \
             golf-lab import-photos)\nerror: no clips found in {}\n",
            still.display(),
            still.display()
        )
    );
}

/// A directory is scanned one level deep for `.mov` in any case, sorted.
#[test]
fn a_directory_gives_its_clips_in_any_case_one_level_deep() {
    let scratch = Scratch::new();
    scratch.clip("img_0002.mov");
    fs::create_dir_all(scratch.path("clips/deeper")).unwrap();
    fs::write(scratch.path("clips/deeper/IMG_0003.MOV"), b"too deep").unwrap();
    fs::write(scratch.path("clips/notes.txt"), b"not a clip").unwrap();
    let request = scratch.request(&[scratch.path("clips")]);
    let mut motion = Clips::blank();
    let (code, out, err) = run(&request, &mut motion, &mut Replay::grid());
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    let decoded: Vec<&str> = motion
        .asked
        .iter()
        .filter(|(_, width)| width.is_none())
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(decoded, [CLIP, "img_0002.mov"]);
}

/// `--into-session`: the frame's PNG is the entry's photo, with the clip's time, the clip is beside
/// it and not in the manifest, the shot holds the replay's offset and names the entry's copy, the
/// entry is analysed and pooled, and deleting it takes the clip with it.
#[test]
fn into_a_session_the_frame_is_the_entrys_photo_with_the_clip_beside_it() {
    let scratch = Scratch::new();
    let clip = scratch.path("clips").join(CLIP);
    let request = scratch.entries_request(std::slice::from_ref(&clip));
    let (code, out, err) = run(&request, &mut Clips::strike(), &mut Replay::grid());
    assert_eq!(err, "");
    assert_eq!(code, 0, "{out}");
    assert!(
        out.starts_with(
            "\nImporting 1 clip(s) into session '2026-10-23' as photo entries (golfer aaron, \
             club 7i) [full_swing]\n"
        ),
        "{out}"
    );
    assert!(
        out.contains("\n      2026-10-23/1: wrote shot_analysis.json"),
        "{out}"
    );

    let png = grid_png(CLIP);
    let digest = hash_image(&png);
    let manifest = scratch.sessions().get_swing(SESSION, "1").expect("entry 1");
    assert_eq!(manifest.roles.len(), 1, "the clip takes no role");
    let photo = &manifest.roles[&Role::ShotScreen];
    assert_eq!(photo.filename, format!("shot_screen.{}.png", &digest[..12]));
    assert_eq!(photo.original_filename, "IMG_0001.png");
    assert_eq!(photo.content_type, "image/png");
    assert_eq!(photo.content_sha256, digest);
    let swing_dir = scratch.path("sessions").join(SESSION).join("1");
    let copy = swing_dir.join(&photo.filename);
    assert_eq!(fs::read(&copy).unwrap(), png);
    let beside = swing_dir.join(format!("shot_screen.{}.mov", &digest[..12]));
    assert_eq!(fs::read(&beside).unwrap(), fs::read(&clip).unwrap());
    let time = |path: &Path| fs::metadata(path).unwrap().modified().unwrap();
    assert_eq!(
        time(&copy),
        time(&clip),
        "the shot's timestamp is the clip's"
    );
    assert_eq!(time(&beside), time(&clip));

    let shot = scratch.store().get(&digest).unwrap().unwrap();
    assert_eq!(shot.impact_offset_h, Some(offset_of_strike()));
    assert!(listed(&shot));
    assert_eq!(shot.shot_id, "2026-10-23-1");
    assert_eq!(shot.timestamp, ocr::screen_timestamp(&clip).unwrap());
    assert_eq!(
        shot.provenance.as_ref().unwrap().image_path,
        Some(copy.display().to_string()),
        "the entry's copy stays, so it is what the shot names"
    );
    let result = load_shot_analysis(&swing_dir).expect("shot_analysis.json");
    assert_eq!(result.photo_sha256.as_deref(), Some(digest.as_str()));

    let corpus = read_corpus(
        &scratch.path("sessions"),
        "aaron",
        EngineVersions {
            installed: ANALYSIS_VERSION,
            comparable_from: COMPARABLE_FROM,
        },
    );
    assert!(corpus.excluded.is_empty(), "{:?}", corpus.excluded);
    let [entry] = corpus.swings.as_slice() else {
        panic!("one entry pooled: {:?}", corpus.swings);
    };
    assert_eq!(
        entry.shot.as_ref().and_then(|shot| shot.impact_offset_h),
        Some(offset_of_strike())
    );

    assert!(scratch.sessions().delete_swing(SESSION, "1").unwrap());
    assert!(!beside.exists(), "a deleted entry takes its clip with it");
}

/// One clip imported twice lands on one entry, since the frame's PNG is the same bytes: the second
/// run reads nothing and says so, and repeats what the replay gave. `--force` reads both again and
/// opens nothing new, and puts back a clip gone from beside the photo. A refused replay reads the
/// same way.
#[test]
fn one_clip_imported_twice_lands_on_one_entry() {
    let scratch = Scratch::new();
    let request = scratch.entries_request(&[scratch.path("clips").join(CLIP)]);
    let mut reader = Replay::grid();
    let (code, _, err) = run(&request, &mut Clips::blank(), &mut reader);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(reader.asked.len(), 1);

    let mut motion = Clips::blank();
    let (code, out, err) = run(&request, &mut motion, &mut reader);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("2026-10-23/1: skipped: this session already holds the photo (--force"),
        "{out}"
    );
    assert!(
        out.contains("impact video: no ball could be found sitting still in the replay"),
        "{out}"
    );
    assert_eq!(reader.asked.len(), 1, "the second run read nothing");
    assert_eq!(
        motion.asked,
        [(CLIP.to_string(), None)],
        "the grid's digest is the lookup, and no replay is read"
    );
    assert_eq!(scratch.sessions().get_session(SESSION).len(), 1);
    assert_eq!(scratch.store().all().len(), 1);

    let digest = hash_image(&grid_png(CLIP));
    let swing_dir = scratch.path("sessions").join(SESSION).join("1");
    let beside = swing_dir.join(format!("shot_screen.{}.mov", &digest[..12]));
    fs::remove_file(&beside).unwrap();
    let mut forced = scratch.entries_request(&[scratch.path("clips").join(CLIP)]);
    forced.options.force_ocr = true;
    let (code, out, err) = run(&forced, &mut Clips::strike(), &mut reader);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert_eq!(reader.asked.len(), 2, "--force reads the entry's copy");
    assert_eq!(scratch.sessions().get_session(SESSION).len(), 1);
    assert!(beside.exists(), "--force puts the clip back");
    let shot = scratch.store().get(&digest).unwrap().unwrap();
    assert_eq!(
        shot.impact_offset_h,
        Some(offset_of_strike()),
        "--force reads the replay again, and the refusal's warning goes"
    );
    assert_eq!(
        shot.provenance.as_ref().unwrap().warnings,
        Vec::<String>::new()
    );
}

/// A clip whose grid gives nothing opens no entry, and its clip is not left behind either.
#[test]
fn a_clip_nothing_is_read_from_opens_no_entry() {
    let scratch = Scratch::new();
    let request = scratch.entries_request(&[scratch.path("clips").join(CLIP)]);
    let (code, out, _) = run(&request, &mut Clips::strike(), &mut Replay::blank());
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("2026-10-23: no photo entry opened, since no shot was read from the photo"),
        "{out}"
    );
    assert!(!scratch.path("sessions").join(SESSION).exists());
    assert!(scratch.store().all().is_empty());
}

/// A dry run reads the grid and the replay, says what each gave, and writes nothing, store-only and
/// into a session alike.
#[test]
fn a_dry_run_reads_both_and_writes_nothing() {
    let scratch = Scratch::new();
    let clip = scratch.path("clips").join(CLIP);
    for mut request in [
        scratch.request(std::slice::from_ref(&clip)),
        scratch.entries_request(std::slice::from_ref(&clip)),
    ] {
        request.options.dry_run = true;
        let mut motion = Clips::blank();
        let (code, out, err) = run(&request, &mut motion, &mut Replay::grid());
        assert_eq!((code, err.as_str()), (0, ""), "{out}");
        assert!(
            out.contains("  IMG_0001.MOV                 dry-run"),
            "{out}"
        );
        assert!(
            out.contains("impact video: no ball could be found sitting still in the replay"),
            "{out}"
        );
        assert!(!out.contains("Store:"), "{out}");
        assert_eq!(motion.asked.len(), 2);
    }
    assert!(!scratch.path("shots").exists());
    assert!(!scratch.path("sessions").exists());
}

/// A current stored shot costs the grid's decode, for its digest, and neither the OCR nor the
/// replay: the still's cache rule. `--force` reads both again, as a new import.
#[test]
fn a_current_stored_shot_costs_no_read_and_no_replay() {
    let scratch = Scratch::new();
    let mut request = scratch.request(&[scratch.path("clips").join(CLIP)]);
    let mut reader = Replay::grid();
    run(&request, &mut Clips::blank(), &mut reader);

    let mut motion = Clips::blank();
    let (code, out, _) = run(&request, &mut motion, &mut reader);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("  IMG_0001.MOV                 cached "),
        "{out}"
    );
    assert!(
        out.contains("impact video: no ball could be found sitting still in the replay"),
        "{out}"
    );
    assert_eq!(reader.asked.len(), 1);
    assert_eq!(motion.asked, [(CLIP.to_string(), None)]);

    request.options.force_ocr = true;
    let mut motion = Clips::strike();
    let (code, out, _) = run(&request, &mut motion, &mut reader);
    assert_eq!(code, 0, "{out}");
    assert_eq!(reader.asked.len(), 2);
    assert_eq!(motion.asked.len(), 2);
    assert!(out.contains("impact video: offset +"), "{out}");
}
