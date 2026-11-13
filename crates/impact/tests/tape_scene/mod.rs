//! Synthetic photos of a STRIKEPRO label stuck on a club face, drawn through a pinhole camera, for
//! the tape reader's tests. [M46 P9, P10, P11, P12]
//!
//! The M46 plan builds the reader on synthetic photos alone, because no photo of a label on a club
//! exists before P29 (P3 finding 10). This module is the one renderer P9 to P12's tests draw with,
//! and P13's end-to-end gate draws its sets with it. It lives under `tests/` in a directory, so each
//! test file that wants it says `mod tape_scene;`, and it is no test crate of its own. Beside the
//! drawing it holds what more than one test file frames a scene with: [`truth`], [`framed`] and
//! [`tilted_poses`] (moved here from `tape_orient.rs` at P11), and [`bottom_at`] (from
//! `tape_mark.rs` at P12).
//!
//! # What a photo holds
//!
//! - **The plane** is the club's face, seen by the camera. Its frame is millimetres, `x` to the
//!   right and `y` up as the camera sees it when unturned (roll 0), and the origin is the middle of
//!   the face.
//! - **The head** is a face outline with a hosel and a shaft leaving its heel, up and away from the
//!   toe at a lie of 62°. A right-handed club seen face-on has its toe on the photo's left (the
//!   plan's C4), so its heel and hosel are on the right. A left-handed one is that mirrored.
//!   [`Scene::shaft`] can leave both out, or draw a second at the toe, for the orientation's
//!   refusals (P10). The shaft lies in the face's plane, which a real one does not (`orient.rs`).
//! - **The label** is the type's measured outline (`LabelType::outline_mm`) with its cross at
//!   [`Scene::label_at_mm`]. Stuck logo-up, its printed frame is the plane's, and stuck logo-down it
//!   is turned half a turn. The labels are sold for either hand and are printed one way, so a
//!   left-handed club's label is never mirrored (P4 finding 5).
//! - **Its printing**, from P3 finding 4, at about the measured sizes: the navy logo centred along
//!   the top edge, its letters 3.2 to 6.2 mm below the edge with a red target in their middle, under
//!   a thin swoosh that rises to [`Scene::logo_gap_mm`] from the edge (where P9 measured them on P3's
//!   flat photo); the red circle and cross at the centre; and each type's black dashes, rings for
//!   the driver and hybrid and a grid for the iron, with the driver's distance text.
//! - **Marks**, darker than the white (P3 finding 8), at printed-frame millimetres.
//!
//! Each pixel is the mean of four samples, so an edge is blended as a camera blends it, and has a
//! little fixed noise and a gentle light gradient across the photo, so no region is perfectly flat.

#![allow(dead_code)] // Each test file that includes this module uses part of it.

use impact::tape::{LabelType, Orientation, STRIKEPRO_DRIVER, STRIKEPRO_HYBRID, STRIKEPRO_IRON};
use impact::Frame;

/// The camera's focal length in pixels, about a phone's main camera over an 800-pixel-wide photo.
pub const FOCAL_PX: f64 = 1000.0;
pub const PHOTO_WIDTH: usize = 800;
pub const PHOTO_HEIGHT: usize = 600;

pub const LABEL_WHITE: [u8; 3] = [236, 236, 230];
pub const NAVY: [u8; 3] = [35, 45, 95];
pub const RED: [u8; 3] = [200, 40, 45];
pub const BLACK: [u8; 3] = [30, 30, 34];
/// A strike's mark: darker than the white and lighter than the printing (P3 finding 8). Its real
/// colour is unknown before P29.
pub const MARK: [u8; 3] = [120, 116, 122];
/// An iron's face, satin steel.
pub const STEEL: [u8; 3] = [158, 160, 165];
/// A driver's face, painted or carbon.
pub const CARBON: [u8; 3] = [42, 42, 48];
/// A graphite shaft.
pub const SHAFT: [u8; 3] = [50, 50, 56];
/// A hitting mat behind the club.
pub const MAT: [u8; 3] = [95, 120, 80];

/// The lie angle the shaft leaves the face at, from the face's horizontal.
const LIE_DEG: f64 = 62.0;

/// How a mark's edge blends into the label, millimetres, so a mark is not stepped where it crosses
/// a pixel.
const MARK_RAMP_MM: f64 = 0.4;

/// A plane posed before a pinhole camera at the origin looking along `+z`: turned by `rotation`
/// (pitch about `x`, then yaw about `y`, then roll about the optical axis) and moved by
/// `translation`, millimetres. The same camera as `tests/tape_geometry.rs`'s.
pub struct Camera {
    rotation: [[f64; 3]; 3],
    translation: [f64; 3],
}

impl Camera {
    pub fn new(
        pitch_deg: f64,
        yaw_deg: f64,
        roll_deg: f64,
        distance_mm: f64,
        shift_mm: [f64; 2],
    ) -> Camera {
        let (a, b, g) = (
            pitch_deg.to_radians(),
            yaw_deg.to_radians(),
            roll_deg.to_radians(),
        );
        let pitch = [
            [1.0, 0.0, 0.0],
            [0.0, a.cos(), -a.sin()],
            [0.0, a.sin(), a.cos()],
        ];
        let yaw = [
            [b.cos(), 0.0, b.sin()],
            [0.0, 1.0, 0.0],
            [-b.sin(), 0.0, b.cos()],
        ];
        let roll = [
            [g.cos(), -g.sin(), 0.0],
            [g.sin(), g.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
        Camera {
            rotation: multiply(&roll, &multiply(&yaw, &pitch)),
            translation: [shift_mm[0], shift_mm[1], distance_mm],
        }
    }

    /// Face-on, unturned, `distance_mm` away.
    pub fn face_on(distance_mm: f64) -> Camera {
        Camera::new(0.0, 0.0, 0.0, distance_mm, [0.0, 0.0])
    }

    /// Turned as [`Camera::new`] turns it, and moved so that the plane point `aim_mm` is the
    /// photo's centre at `distance_mm`: how a test frames the hosel and shaft beside the label.
    /// [M46 P10]
    pub fn aimed(
        pitch_deg: f64,
        yaw_deg: f64,
        roll_deg: f64,
        distance_mm: f64,
        aim_mm: [f64; 2],
    ) -> Camera {
        let mut camera = Camera::new(pitch_deg, yaw_deg, roll_deg, distance_mm, [0.0, 0.0]);
        let (u, v) = (camera.column(0), camera.column(1));
        let aim: [f64; 3] = std::array::from_fn(|k| aim_mm[0] * u[k] + aim_mm[1] * v[k]);
        camera.translation = [-aim[0], -aim[1], distance_mm - aim[2]];
        camera
    }

    /// The angle between the plane's normal and the optical axis: 0 face-on.
    pub fn tilt_deg(&self) -> f64 {
        self.rotation[2][2].acos().to_degrees()
    }

    fn column(&self, c: usize) -> [f64; 3] {
        [0, 1, 2].map(|r| self.rotation[r][c])
    }

    /// Where a plane point lands in the photo. The photo's `y` runs down, the world's up.
    pub fn project(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        let (u, v, t) = (self.column(0), self.column(1), self.translation);
        let p: [f64; 3] = std::array::from_fn(|k| x * u[k] + y * v[k] + t[k]);
        [
            PHOTO_WIDTH as f64 / 2.0 + FOCAL_PX * p[0] / p[2],
            PHOTO_HEIGHT as f64 / 2.0 - FOCAL_PX * p[1] / p[2],
        ]
    }

    /// The plane point a photo point sees: its ray met with the plane.
    pub fn unproject(&self, [px, py]: [f64; 2]) -> [f64; 2] {
        let ray = [
            (px - PHOTO_WIDTH as f64 / 2.0) / FOCAL_PX,
            -(py - PHOTO_HEIGHT as f64 / 2.0) / FOCAL_PX,
            1.0,
        ];
        let normal = self.column(2);
        let t = self.translation;
        let s = dot(normal, t) / dot(normal, ray);
        let offset: [f64; 3] = std::array::from_fn(|k| s * ray[k] - t[k]);
        [dot(self.column(0), offset), dot(self.column(1), offset)]
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn multiply(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

/// The club's hand is the reader's own type, so a scene's hand is what a test passes to it as the
/// golfer's. [M46 P10]
pub use impact::tape::Handedness as Hand;

/// Which way the scene's club lies across its label, from the scene alone. In the face's plane the
/// toe is toward `−x` on a right-handed club and `+x` on a left-handed one (C4), and the top line is
/// `+y`. A label stuck logo-down has its printed frame turned half a turn from the plane's. [M46
/// P10, moved here from `tape_orient.rs` at P11]
pub fn truth(scene: &Scene) -> Orientation {
    let toe = match scene.hand {
        Hand::Right => -1.0,
        Hand::Left => 1.0,
    };
    let turn = if scene.logo_up { 1.0 } else { -1.0 };
    Orientation {
        toe_printed_right: toe * turn > 0.0,
        top_line_printed_top: turn > 0.0,
    }
}

/// A scene of `label` on the head its type suits, the camera aimed beside the label toward the
/// heel and up, so the hosel and some shaft are in frame, as the tape camera's frame will ask. [M46
/// P10, moved here from `tape_orient.rs` at P11]
pub fn framed(
    label: &'static LabelType,
    hand: Hand,
    logo_up: bool,
    [pitch, yaw, roll]: [f64; 3],
    distance_mm: f64,
) -> Scene {
    let heel = match hand {
        Hand::Right => 1.0,
        Hand::Left => -1.0,
    };
    let camera = Camera::aimed(pitch, yaw, roll, distance_mm, [heel * 12.0, 15.0]);
    let mut scene = Scene::new(label, camera);
    scene.hand = hand;
    scene.logo_up = logo_up;
    scene
}

/// One of the six poses up to 30° off square that P10 oriented, each with the aim that keeps the
/// whole label and some shaft in the 800 × 600 photo. [M46 P10, shared at P11]
#[derive(Debug, Clone, Copy)]
pub struct Pose {
    pub label: &'static LabelType,
    pub hand: Hand,
    pub logo_up: bool,
    /// Pitch, yaw and roll, as [`Camera::new`] takes them.
    pub turn: [f64; 3],
    pub aim_mm: [f64; 2],
}

impl Pose {
    /// The pose's scene at 200 mm, unmarked.
    pub fn scene(&self) -> Scene {
        let mut scene = framed(self.label, self.hand, self.logo_up, self.turn, 200.0);
        let [pitch, yaw, roll] = self.turn;
        scene.camera = Camera::aimed(pitch, yaw, roll, 200.0, self.aim_mm);
        scene
    }
}

/// Every type, both hands, both ways up, tilted 28.5° to 30° and turned −90° to 180°. Pose 2's yaw
/// and roll swing the shaft toward the photo's top edge, so it is aimed further along the shaft
/// than the rest (P10 finding 8).
pub fn tilted_poses() -> [Pose; 6] {
    let pose = |label, hand, logo_up, turn, aim_mm| Pose {
        label,
        hand,
        logo_up,
        turn,
        aim_mm,
    };
    [
        pose(
            &STRIKEPRO_DRIVER,
            Hand::Right,
            true,
            [25.0, -15.0, 90.0],
            [12.0, 15.0],
        ),
        pose(
            &STRIKEPRO_DRIVER,
            Hand::Left,
            false,
            [-12.0, 26.0, 180.0],
            [-12.0, 15.0],
        ),
        pose(
            &STRIKEPRO_HYBRID,
            Hand::Left,
            true,
            [0.0, -30.0, -40.0],
            [-22.0, 26.0],
        ),
        pose(
            &STRIKEPRO_HYBRID,
            Hand::Right,
            false,
            [10.0, 28.0, -90.0],
            [12.0, 15.0],
        ),
        pose(
            &STRIKEPRO_IRON,
            Hand::Right,
            true,
            [20.0, 21.0, 120.0],
            [12.0, 15.0],
        ),
        pose(
            &STRIKEPRO_IRON,
            Hand::Left,
            true,
            [-25.0, 15.0, 65.0],
            [-12.0, 15.0],
        ),
    ]
}

/// What leaves the head at the hosel: the reader's only cue to which end is the heel. [M46 P10]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shaft {
    /// The hosel and a graphite shaft, at the heel.
    AtHeel,
    /// Neither, as a photo framed too tight to hold them shows the head.
    Missing,
    /// A hosel and shaft at each end, as a dark bar leaving the toe too (a shadow, or another
    /// club) would make it: no end is the heel.
    AtBothEnds,
}

/// Which club's face is drawn around the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Head {
    /// A steel iron's face, a little larger than the iron label, taller at the toe.
    Iron,
    /// A driver's dark face, larger than the driver label.
    Driver,
    /// No club: the label on the mat.
    None,
}

/// A white shape drawn where the label would be, with no printing: what the finder must refuse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Card {
    /// A rectangle about the label's cross, `width_mm` by `height_mm`.
    Rectangle {
        width_mm: f64,
        height_mm: f64,
    },
    Disc {
        radius_mm: f64,
    },
}

/// A strike's mark: a disc darker than the label, at printed-frame millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub centre_mm: [f64; 2],
    pub radius_mm: f64,
}

/// One photo to draw.
pub struct Scene {
    pub label: &'static LabelType,
    pub camera: Camera,
    pub hand: Hand,
    /// The label stuck with its logo toward the top line. `false` turns it half a turn.
    pub logo_up: bool,
    pub head: Head,
    /// What leaves the head at the hosel. Drawn only with a head.
    pub shaft: Shaft,
    /// Behind the club: [`MAT`] unless a test needs another.
    pub background: [u8; 3],
    /// The label's cross in the plane's frame.
    pub label_at_mm: [f64; 2],
    /// A second label of the same type, its cross here, for a photo with two.
    pub second_label_at_mm: Option<[f64; 2]>,
    /// No label at all, only the head.
    pub no_label: bool,
    /// A plain white shape in the label's place.
    pub card: Option<Card>,
    pub marks: Vec<Mark>,
    /// How near the label's top edge the logo's swoosh comes, at its right-hand end, millimetres:
    /// about 1 (P3 finding 7). Below 0 it crosses the edge and notches the label's white, as P3's
    /// blurred outline notched there.
    pub logo_gap_mm: f64,
}

impl Scene {
    /// A right-handed club, its label logo-up at the face's middle, on the head its label type
    /// suits, with no marks.
    pub fn new(label: &'static LabelType, camera: Camera) -> Scene {
        Scene {
            label,
            camera,
            hand: Hand::Right,
            logo_up: true,
            head: if label.id == "strikepro_driver" {
                Head::Driver
            } else {
                Head::Iron
            },
            shaft: Shaft::AtHeel,
            background: MAT,
            label_at_mm: [0.0, 0.0],
            second_label_at_mm: None,
            no_label: false,
            card: None,
            marks: Vec::new(),
            logo_gap_mm: 1.0,
        }
    }

    /// The plane point of a printed-frame point on the label.
    pub fn plane_of(&self, printed: [f64; 2]) -> [f64; 2] {
        let [x, y] = if self.logo_up {
            printed
        } else {
            [-printed[0], -printed[1]]
        };
        [self.label_at_mm[0] + x, self.label_at_mm[1] + y]
    }

    /// Where a printed-frame point on the label lands in the photo.
    pub fn photo_of(&self, printed: [f64; 2]) -> [f64; 2] {
        self.camera.project(self.plane_of(printed))
    }

    /// The label's box corners in the photo, top-left, top-right, bottom-right, bottom-left of the
    /// printed frame: what the finder should return.
    pub fn true_corners(&self) -> [[f64; 2]; 4] {
        self.label.box_corners_mm().map(|c| self.photo_of(c))
    }

    /// The photo. A pixel whose four corners see one colour is that colour, and any other is the
    /// mean of four samples inside it, which is what a pixel straddling an edge needs. Sampling
    /// every pixel four times costs three times as long in a debug build and draws the same flat
    /// regions.
    pub fn draw(&self) -> Frame {
        let printing = Printing::of(self.label, self.logo_gap_mm);
        let sample = |x: f64, y: f64| self.colour_at(self.camera.unproject([x, y]), &printing);
        let corners: Vec<[u8; 3]> = (0..=PHOTO_HEIGHT)
            .flat_map(|row| (0..=PHOTO_WIDTH).map(move |column| (column, row)))
            .map(|(column, row)| sample(column as f64, row as f64))
            .collect();
        let corner = |column: usize, row: usize| corners[row * (PHOTO_WIDTH + 1) + column];
        let mut rgb = Vec::with_capacity(PHOTO_WIDTH * PHOTO_HEIGHT * 3);
        for row in 0..PHOTO_HEIGHT {
            for column in 0..PHOTO_WIDTH {
                let first = corner(column, row);
                let flat = [(1, 0), (0, 1), (1, 1)]
                    .iter()
                    .all(|&(dx, dy)| corner(column + dx, row + dy) == first);
                let mut sum = [0.0; 3];
                if flat {
                    sum = first.map(f64::from);
                } else {
                    for (dx, dy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                        let colour = sample(column as f64 + dx, row as f64 + dy);
                        for (total, value) in sum.iter_mut().zip(colour) {
                            *total += f64::from(value) / 4.0;
                        }
                    }
                }
                let light = 1.0 + 0.04 * (column as f64 / PHOTO_WIDTH as f64 - 0.5)
                    - 0.03 * (row as f64 / PHOTO_HEIGHT as f64 - 0.5);
                let grain = noise(column, row);
                rgb.extend(sum.map(|v| (v * light + grain).round().clamp(0.0, 255.0) as u8));
            }
        }
        Frame {
            width: PHOTO_WIDTH,
            height: PHOTO_HEIGHT,
            rgb,
        }
    }

    fn colour_at(&self, plane: [f64; 2], printing: &Printing) -> [u8; 3] {
        if !self.no_label {
            let labels = std::iter::once(self.label_at_mm).chain(self.second_label_at_mm);
            for at in labels {
                let d = [plane[0] - at[0], plane[1] - at[1]];
                let printed = if self.logo_up { d } else { [-d[0], -d[1]] };
                if let Some(card) = self.card {
                    if card.holds(printed) {
                        return LABEL_WHITE;
                    }
                } else if in_box(self.label, printed) && inside(self.label.outline_mm, printed) {
                    return self.marked(printed, printing.colour_at(printed));
                }
            }
        }
        // The head, in a frame with the heel to the right whichever hand the club is.
        let heel_right = match self.hand {
            Hand::Right => plane,
            Hand::Left => [-plane[0], plane[1]],
        };
        let (face, hosel_base, colour): (&[[f64; 2]], [f64; 2], [u8; 3]) = match self.head {
            Head::Iron => (&IRON_FACE, [40.0, 12.0], STEEL),
            Head::Driver => (&DRIVER_FACE, [52.0, 18.0], CARBON),
            Head::None => return self.background,
        };
        let up = [LIE_DEG.to_radians().cos(), LIE_DEG.to_radians().sin()];
        // A point along the hosel's line from its base, in a frame with that hosel's end to the
        // right: `from..=to` along it and within `half_width` across.
        let along = |point: [f64; 2], from: f64, to: f64, half_width: f64| {
            let rel = [point[0] - hosel_base[0], point[1] - hosel_base[1]];
            let t = rel[0] * up[0] + rel[1] * up[1];
            let across = (rel[0] * up[1] - rel[1] * up[0]).abs();
            (from..=to).contains(&t) && across <= half_width
        };
        let toe_right = [-heel_right[0], heel_right[1]];
        let hosels: &[[f64; 2]] = match self.shaft {
            Shaft::AtHeel => &[heel_right],
            Shaft::Missing => &[],
            Shaft::AtBothEnds => &[heel_right, toe_right],
        };
        if hosels.iter().any(|&at| along(at, 30.0, 1000.0, 4.5)) {
            SHAFT
        } else if hosels.iter().any(|&at| along(at, 0.0, 30.0, 7.0)) || inside(face, heel_right) {
            colour
        } else {
            self.background
        }
    }

    /// `base` with any mark at `printed` laid over it.
    fn marked(&self, printed: [f64; 2], base: [u8; 3]) -> [u8; 3] {
        let mut colour = base;
        for mark in &self.marks {
            let from = (printed[0] - mark.centre_mm[0]).hypot(printed[1] - mark.centre_mm[1]);
            let ink = ((mark.radius_mm + MARK_RAMP_MM / 2.0 - from) / MARK_RAMP_MM).clamp(0.0, 1.0);
            if ink > 0.0 {
                colour = std::array::from_fn(|k| {
                    (f64::from(colour[k]) * (1.0 - ink) + f64::from(MARK[k]) * ink).round() as u8
                });
            }
        }
        colour
    }
}

impl Card {
    fn holds(self, [x, y]: [f64; 2]) -> bool {
        match self {
            Card::Rectangle {
                width_mm,
                height_mm,
            } => x.abs() <= width_mm / 2.0 && y.abs() <= height_mm / 2.0,
            Card::Disc { radius_mm } => x.hypot(y) <= radius_mm,
        }
    }
}

/// An iron's face in a frame with the heel to the right: the toe taller than the heel, as an iron's
/// is, and clear of the iron label's box on every side.
const IRON_FACE: [[f64; 2]; 10] = [
    [-46.0, -22.0],
    [38.0, -22.0],
    [44.0, -17.0],
    [46.0, 14.0],
    [42.0, 22.0],
    [-30.0, 32.0],
    [-42.0, 30.0],
    [-49.0, 20.0],
    [-50.0, -12.0],
    [-48.0, -19.0],
];

/// A driver's face in a frame with the heel to the right, clear of the driver label.
const DRIVER_FACE: [[f64; 2]; 10] = [
    [-62.0, -30.0],
    [50.0, -30.0],
    [58.0, -22.0],
    [61.0, 8.0],
    [54.0, 24.0],
    [20.0, 33.0],
    [-30.0, 34.0],
    [-56.0, 26.0],
    [-65.0, 8.0],
    [-66.0, -14.0],
];

/// Whether a printed-frame point is inside the label's box, which holds its outline: a cheap test
/// before the outline's own.
fn in_box(label: &LabelType, [x, y]: [f64; 2]) -> bool {
    let [[left, top], _, [right, bottom], _] = label.box_corners_mm();
    (left - 0.5..=right + 0.5).contains(&x) && (bottom - 0.5..=top + 0.5).contains(&y)
}

/// Whether a point is inside a polygon, by the crossing count.
fn inside(polygon: &[[f64; 2]], [x, y]: [f64; 2]) -> bool {
    let mut crossings = false;
    for (i, &[x0, y0]) in polygon.iter().enumerate() {
        let [x1, y1] = polygon[(i + 1) % polygon.len()];
        if (y0 > y) != (y1 > y) && x < x0 + (y - y0) * (x1 - x0) / (y1 - y0) {
            crossings = !crossings;
        }
    }
    crossings
}

/// A fixed grain of −3 to +3 levels, the same on every channel, by a hash of the pixel.
fn noise(column: usize, row: usize) -> f64 {
    let mut h = (column as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (row as u64);
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 31;
    (h % 7) as f64 - 3.0
}

/// One label type's printing, in its printed frame, laid out once a photo.
struct Printing {
    /// The logo's letters, each a box: left, right, bottom, top.
    letters: Vec<[f64; 4]>,
    /// The logo's red target: its centre and radius.
    target: ([f64; 2], f64),
    /// The swoosh: its ends' `x`, and its depth below the top edge at each.
    swoosh: ([f64; 2], [f64; 2]),
    /// The red circle's radius about the cross.
    ring_mm: f64,
    kind: Dashes,
    label: &'static LabelType,
}

enum Dashes {
    /// Dashed rings about the cross, and the distance text.
    Driver,
    /// Dashed ellipses about the cross, as semi-axes.
    Hybrid,
    /// A dashed grid through the cross.
    Iron,
}

impl Printing {
    /// The letters' rows below the box's top edge and their half-span across are as P9 measured
    /// them on P3's flat photo: 3.2 to 6.2 mm down on all three labels, and about ±16 mm across on
    /// the hybrid and iron labels and ±19 mm on the driver's.
    fn of(label: &'static LabelType, gap_mm: f64) -> Printing {
        let (half, ring_mm, kind) = match label.id {
            "strikepro_driver" => (19.0, 23.1 / 2.0, Dashes::Driver),
            "strikepro_hybrid" => (16.0, 18.3 / 2.0, Dashes::Hybrid),
            _ => (16.0, 18.3 / 2.0, Dashes::Iron),
        };
        let top = label.centre_from_top_mm;
        let (letter, space, target_radius) = (1.0, 0.8, 2.0);
        let mut letters = Vec::new();
        let mut x = -half;
        while x + letter <= half {
            // Clear of the target in the middle.
            if x + letter < -target_radius - 0.4 || x > target_radius + 0.4 {
                letters.push([x, x + letter, top - 6.2, top - 3.2]);
            }
            x += letter + space;
        }
        Printing {
            letters,
            target: ([0.0, top - 4.7], target_radius),
            swoosh: ([-half + 2.0, half - 2.0], [2.6, gap_mm]),
            ring_mm,
            kind,
            label,
        }
    }

    fn colour_at(&self, [x, y]: [f64; 2]) -> [u8; 3] {
        let line = 0.3;
        let cross = (x.abs() <= 5.0 && y.abs() <= line) || (y.abs() <= 5.0 && x.abs() <= line);
        let ring = (x.hypot(y) - self.ring_mm).abs() <= line;
        let (centre, radius) = self.target;
        let from_target = (x - centre[0]).hypot(y - centre[1]);
        let target = (from_target - radius).abs() <= 0.35 || from_target <= 0.7;
        if cross || ring || target {
            return RED;
        }
        let letter = self
            .letters
            .iter()
            .any(|&[l, r, b, t]| (l..=r).contains(&x) && (b..=t).contains(&y));
        // The swoosh rises from the letters' left end to near the top edge at their right, 0.5 mm
        // thick, its depth measured down from the edge above it.
        let ([from, to], [deep, shallow]) = self.swoosh;
        let swoosh = (from..=to).contains(&x) && {
            let along = (to - x) / (to - from);
            let depth = shallow + (deep - shallow) * along * along;
            ((top_edge(self.label, x) - depth - 0.25) - y).abs() <= 0.25
        };
        if letter || swoosh {
            return NAVY;
        }
        if self.dashed(x, y) {
            return BLACK;
        }
        LABEL_WHITE
    }

    /// The type's black dashes, 2 mm on and 1 mm off, 0.45 mm wide, and the driver's text.
    ///
    /// Placed as P9 read them off P3's flat photo, where they decide what lies in the bands the
    /// finder compares (the logo's and the one as deep in from the bottom edge). On the driver
    /// label, two dashed ellipses about the cross, the outer one 2 mm above the bottom edge and
    /// meeting the logo at the top, and its text: `-7%` above the circle, `-15%` either side, and
    /// `7% Loss of Distance` about 12 mm below the cross. On the hybrid label, two dashed ellipses a
    /// little below the cross, crossing its axis at ±19.5 and ±28.5 mm (P3 finding 4), the outer one
    /// 2 mm above the bottom edge. On the iron label, a dashed grid through the cross and 10.5 mm
    /// either side of it, which puts its bottom line in the bottom band, as the real one is.
    fn dashed(&self, x: f64, y: f64) -> bool {
        let half_width = 0.225;
        let on = |along: f64| along.rem_euclid(3.0) < 2.0;
        // A dashed ellipse: its centre's height and its semi-axes.
        let ellipse = |(cy, a, b): (f64, f64, f64)| {
            let (u, v) = (x / a, (y - cy) / b);
            (u.hypot(v) - 1.0).abs() * a.min(b) <= half_width && on(v.atan2(u) * (a + b) / 2.0)
        };
        match self.kind {
            Dashes::Driver => {
                let rings = [(0.0, 32.5, 22.5), (0.0, 20.3, 16.2)]
                    .into_iter()
                    .any(ellipse);
                let text = [
                    [-2.5, 2.5, 13.0, 15.0],
                    [-34.0, -28.0, -1.0, 1.0],
                    [28.0, 34.0, -1.0, 1.0],
                    [-9.0, 9.0, -12.8, -11.2],
                ]
                .iter()
                .any(|&[l, r, b, t]| (l..=r).contains(&x) && (b..=t).contains(&y) && on(x * 2.0));
                rings || text
            }
            Dashes::Hybrid => [(-1.65, 28.6, 13.25), (-1.25, 19.75, 11.65)]
                .into_iter()
                .any(ellipse),
            Dashes::Iron => {
                let rows = [-10.5, 0.0, 10.5]
                    .iter()
                    .any(|&row| (y - row).abs() <= half_width && x.abs() <= 33.0 && on(x));
                let columns = [-18.5, 18.5].iter().any(|&column| {
                    (x - column).abs() <= half_width && (-13.0..=21.0).contains(&y) && on(y)
                });
                rows || columns
            }
        }
    }
}

/// The outline's lowest point at printed `x`: where a vertical line there meets its bottom edge.
/// [M46 P11, moved here from `tape_mark.rs` at P12]
pub fn bottom_at(label: &LabelType, x: f64) -> f64 {
    let outline = label.outline_mm;
    let mut bottom = f64::INFINITY;
    for (i, &[x0, y0]) in outline.iter().enumerate() {
        let [x1, y1] = outline[(i + 1) % outline.len()];
        if (x0 <= x) != (x1 <= x) {
            bottom = bottom.min(y0 + (x - x0) * (y1 - y0) / (x1 - x0));
        }
    }
    bottom
}

/// The outline's top at `x` in the printed frame: the highest point where a vertical line there
/// meets it.
fn top_edge(label: &LabelType, x: f64) -> f64 {
    let outline = label.outline_mm;
    let mut top = f64::NEG_INFINITY;
    for (i, &[x0, y0]) in outline.iter().enumerate() {
        let [x1, y1] = outline[(i + 1) % outline.len()];
        if (x0 <= x) == (x1 <= x) || x0 == x1 {
            continue;
        }
        top = top.max(y0 + (x - x0) * (y1 - y0) / (x1 - x0));
    }
    top
}

/// Writes `frame` as a PNG at `path`, uncompressed, so a drawn photo can be looked at or handed to
/// `golf-lab read-tape --review`. This crate takes no dependency, `image` included, so the format's
/// few parts are written here: stored deflate blocks, Adler-32 and CRC-32.
pub fn write_png(frame: &Frame, path: &std::path::Path) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity((frame.width * 3 + 1) * frame.height);
    for row in frame.rgb.chunks(frame.width * 3) {
        raw.push(0); // no filter
        raw.extend_from_slice(row);
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        zlib.extend(len.to_le_bytes());
        zlib.extend((!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend(((b << 16) | a).to_be_bytes());
    let mut header = Vec::new();
    header.extend((frame.width as u32).to_be_bytes());
    header.extend((frame.height as u32).to_be_bytes());
    header.extend([8, 2, 0, 0, 0]); // 8-bit RGB
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    for (kind, data) in [(b"IHDR", header), (b"IDAT", zlib), (b"IEND", Vec::new())] {
        png.extend((data.len() as u32).to_be_bytes());
        let start = png.len();
        png.extend_from_slice(kind);
        png.extend_from_slice(&data);
        let crc = crc32(&png[start..]);
        png.extend(crc.to_be_bytes());
    }
    std::fs::write(path, png)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// When `TAPE_SCENE_DIR` names a directory, keeps `frame` there as `<name>.png`, for a person to
/// look at or to run `golf-lab read-tape --review` over. Unset, as in every ordinary run, it does
/// nothing.
pub fn keep(frame: &Frame, name: &str) {
    if let Some(dir) = std::env::var_os("TAPE_SCENE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("TAPE_SCENE_DIR can be made");
        write_png(frame, &dir.join(format!("{name}.png"))).expect("the photo is written");
    }
}
