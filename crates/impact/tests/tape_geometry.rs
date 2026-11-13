//! The tape reader's geometry, on synthetic photos with no oracle but the camera that drew them.
//! [M46 P7]
//!
//! Each photo is drawn by casting a ray through every pixel's centre onto a label plane posed in
//! front of a pinhole camera: tilted up to 30° from face-on, turned in the image, and moved nearer
//! or further. So the homography under test is never what drew the photo it is tested on. The label
//! is drawn as its box (`LabelType::box_corners_mm`), white on a grey head, with one dark dot at a
//! known place in the printed frame. Finding the box is P9's, so the corners handed to
//! `Homography::from_corners` are the camera's own projections of the box's.

use impact::tape::{
    rectify, Crop, CropGrid, Degenerate, Homography, LabelType, CROP_PX_PER_MM, STRIKEPRO_DRIVER,
    STRIKEPRO_HYBRID, STRIKEPRO_IRON,
};
use impact::Frame;

/// The camera's focal length in pixels, about a phone's main camera over a 1000-pixel-wide photo.
const FOCAL_PX: f64 = 1100.0;
const PHOTO_WIDTH: usize = 1000;
const PHOTO_HEIGHT: usize = 800;

/// The dot: its radius, and the width of the ramp from ink to label across its edge, so the drawn
/// photo is not stepped where the dot's edge crosses a pixel.
const DOT_RADIUS_MM: f64 = 3.5;
const DOT_RAMP_MM: f64 = 0.4;

const LABEL_WHITE: [u8; 3] = [236, 236, 230];
const INK: [u8; 3] = [40, 40, 48];
/// The club head around the label. Lighter than the ink-weighting threshold, so that crop pixels
/// blending the label's edge with the head are never taken for the dot.
const HEAD: [u8; 3] = [150, 152, 156];

/// How far the dot is recovered from, the phase's done-when.
const RECOVERED_MM: f64 = 0.1;

/// A label plane posed before a pinhole camera at the origin looking along `+z`. The label's `x`
/// and `y` are its printed frame. Its plane is turned by `rotation` (pitch about `x`, then yaw
/// about `y`, then roll about the optical axis) and moved by `translation`, in millimetres.
struct Camera {
    rotation: [[f64; 3]; 3],
    translation: [f64; 3],
}

impl Camera {
    fn new(
        pitch_deg: f64,
        yaw_deg: f64,
        roll_deg: f64,
        distance_mm: f64,
        shift_mm: [f64; 2],
    ) -> Self {
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

    /// The angle between the label's normal and the optical axis: 0 face-on.
    fn tilt_deg(&self) -> f64 {
        self.rotation[2][2].acos().to_degrees()
    }

    fn column(&self, c: usize) -> [f64; 3] {
        [0, 1, 2].map(|r| self.rotation[r][c])
    }

    /// Where a printed-frame point lands in the photo. The photo's `y` runs down, the world's up.
    fn project(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        let (u, v, t) = (self.column(0), self.column(1), self.translation);
        let p: [f64; 3] = std::array::from_fn(|k| x * u[k] + y * v[k] + t[k]);
        [
            PHOTO_WIDTH as f64 / 2.0 + FOCAL_PX * p[0] / p[2],
            PHOTO_HEIGHT as f64 / 2.0 - FOCAL_PX * p[1] / p[2],
        ]
    }

    /// The printed-frame point a photo point sees: its ray met with the label's plane.
    fn unproject(&self, [px, py]: [f64; 2]) -> [f64; 2] {
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

/// Whether a printed-frame point is inside the label's box, at least `margin_mm` from its edges.
fn in_box(label: &LabelType, [x, y]: [f64; 2], margin_mm: f64) -> bool {
    let [[left, top], _, [right, bottom], _] = label.box_corners_mm();
    x >= left + margin_mm
        && x <= right - margin_mm
        && y >= bottom + margin_mm
        && y <= top - margin_mm
}

struct Scene {
    label: &'static LabelType,
    camera: Camera,
    dot_mm: [f64; 2],
}

/// Seven poses over the three label types: two face-on, and five tilted 28.6° to 30° from face-on,
/// each about a different axis. They are turned in the image from −90° to 180°, and stand 110 to
/// 200 mm from the camera, so a millimetre of label spans 5.5 to 10 pixels of photo.
fn scenes() -> Vec<Scene> {
    vec![
        Scene {
            label: &STRIKEPRO_DRIVER,
            camera: Camera::new(0.0, 0.0, 0.0, 150.0, [0.0, 0.0]),
            dot_mm: [12.3, -4.7],
        },
        Scene {
            label: &STRIKEPRO_DRIVER,
            camera: Camera::new(30.0, 0.0, 15.0, 170.0, [-5.0, 8.0]),
            dot_mm: [-20.1, 8.2],
        },
        Scene {
            label: &STRIKEPRO_HYBRID,
            camera: Camera::new(0.0, -30.0, -40.0, 150.0, [4.0, -3.0]),
            dot_mm: [21.0, -7.5],
        },
        Scene {
            label: &STRIKEPRO_IRON,
            camera: Camera::new(20.0, 21.0, 120.0, 150.0, [0.0, 0.0]),
            dot_mm: [-17.4, 6.1],
        },
        Scene {
            label: &STRIKEPRO_IRON,
            camera: Camera::new(-25.0, 15.0, 180.0, 200.0, [6.0, 4.0]),
            dot_mm: [8.0, -9.0],
        },
        Scene {
            label: &STRIKEPRO_HYBRID,
            camera: Camera::new(10.0, 28.0, -90.0, 160.0, [-3.0, 5.0]),
            dot_mm: [-25.0, 3.0],
        },
        Scene {
            label: &STRIKEPRO_IRON,
            camera: Camera::new(0.0, 0.0, 0.0, 110.0, [0.0, 0.0]),
            dot_mm: [0.0, 0.0],
        },
    ]
}

/// The scene's photo: each pixel's centre cast onto the label's plane, one sample a pixel.
fn draw(scene: &Scene) -> Frame {
    let mut rgb = Vec::with_capacity(PHOTO_WIDTH * PHOTO_HEIGHT * 3);
    for row in 0..PHOTO_HEIGHT {
        for column in 0..PHOTO_WIDTH {
            let mm = scene
                .camera
                .unproject([column as f64 + 0.5, row as f64 + 0.5]);
            let colour = if in_box(scene.label, mm, 0.0) {
                let from_dot = (mm[0] - scene.dot_mm[0]).hypot(mm[1] - scene.dot_mm[1]);
                let paper = ((from_dot - (DOT_RADIUS_MM - DOT_RAMP_MM / 2.0)) / DOT_RAMP_MM)
                    .clamp(0.0, 1.0);
                std::array::from_fn(|k| {
                    let (ink, white) = (f64::from(INK[k]), f64::from(LABEL_WHITE[k]));
                    (ink + (white - ink) * paper).round() as u8
                })
            } else {
                HEAD
            };
            rgb.extend(colour);
        }
    }
    Frame {
        width: PHOTO_WIDTH,
        height: PHOTO_HEIGHT,
        rgb,
    }
}

fn luminance([r, g, b]: [u8; 3]) -> f64 {
    0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)
}

/// The ink-weighted centroid of the crop, in printed-frame millimetres: each pixel weighted by how
/// far its luminance is below 128, over pixels at least a millimetre inside the box. It reads only
/// the crop, never where the dot was drawn.
fn ink_centroid_mm(crop: &Crop, label: &LabelType) -> [f64; 2] {
    let (mut total, mut sum_x, mut sum_y) = (0.0, 0.0, 0.0);
    for row in 0..crop.grid.height {
        for column in 0..crop.grid.width {
            let at = row * crop.grid.width + column;
            let centre = [column as f64 + 0.5, row as f64 + 0.5];
            if crop.outside[at] || !in_box(label, crop.grid.to_label_mm(centre), 1.0) {
                continue;
            }
            let pixel = [0, 1, 2].map(|k| crop.frame.rgb[at * 3 + k]);
            let weight = (128.0 - luminance(pixel)).max(0.0);
            total += weight;
            sum_x += weight * centre[0];
            sum_y += weight * centre[1];
        }
    }
    assert!(total > 0.0, "no ink in the crop");
    crop.grid.to_label_mm([sum_x / total, sum_y / total])
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The homography from the camera's projections of the label's box corners.
fn homography_of(scene: &Scene) -> Homography {
    let corners = scene.label.box_corners_mm();
    Homography::from_corners(corners.map(|c| scene.camera.project(c)), corners)
        .expect("a label seen from in front makes a homography")
}

#[test]
fn every_pose_is_within_thirty_degrees_of_face_on() {
    for scene in scenes() {
        assert!(
            scene.camera.tilt_deg() <= 30.0 + 1e-9,
            "{}",
            scene.camera.tilt_deg()
        );
    }
    // And all but the two face-on poses are within 1.5° of the limit, so the limit is exercised.
    let steep = scenes()
        .iter()
        .filter(|scene| scene.camera.tilt_deg() >= 28.5)
        .count();
    assert_eq!(steep, 5);
}

#[test]
fn a_dot_under_perspective_is_recovered_within_a_tenth_of_a_millimetre() {
    for (index, scene) in scenes().iter().enumerate() {
        let photo = draw(scene);
        let h = homography_of(scene);
        let crop = rectify(&photo, &h, scene.label, CROP_PX_PER_MM);
        assert!(
            crop.outside.iter().all(|&outside| !outside),
            "scene {index}: the whole label is in the photo"
        );
        let found = ink_centroid_mm(&crop, scene.label);
        let miss = distance(found, scene.dot_mm);
        assert!(
            miss <= RECOVERED_MM,
            "scene {index}: dot at {:?} found at {found:?}, {miss:.4} mm off",
            scene.dot_mm
        );
        // The dot's own photo position maps to it to rounding: the camera's map is a homography,
        // and four exact corners fix it.
        let mapped = h.apply(scene.camera.project(scene.dot_mm)).unwrap();
        assert!(
            distance(mapped, scene.dot_mm) < 1e-9,
            "scene {index}: {mapped:?}"
        );
    }
}

#[test]
fn each_corner_maps_to_its_label_corner() {
    for scene in scenes() {
        let h = homography_of(&scene);
        for corner in scene.label.box_corners_mm() {
            let photo = scene.camera.project(corner);
            assert!(distance(h.apply(photo).unwrap(), corner) < 1e-9);
            assert!(distance(h.inverse().apply(corner).unwrap(), photo) < 1e-9);
        }
    }
}

#[test]
fn the_inverse_undoes_the_map_within_a_billionth() {
    for scene in scenes() {
        let h = homography_of(&scene);
        // Over the whole photo, a grid of points every 50 pixels, and back.
        for row in 0..=PHOTO_HEIGHT / 50 {
            for column in 0..=PHOTO_WIDTH / 50 {
                let photo = [column as f64 * 50.0, row as f64 * 50.0];
                // A point beyond the horizon has no image, and none of these poses puts one in
                // the photo.
                let label = h
                    .apply(photo)
                    .expect("the whole photo is in front of the horizon");
                let back = h.inverse().apply(label).unwrap();
                assert!(
                    distance(back, photo) < 1e-9,
                    "{photo:?} came back as {back:?}"
                );
            }
        }
        // And over the label's box, every 2 mm, the other way round.
        let [[left, top], _, [right, bottom], _] = scene.label.box_corners_mm();
        let mut y = bottom;
        while y <= top {
            let mut x = left;
            while x <= right {
                let photo = h.inverse().apply([x, y]).unwrap();
                let back = h.apply(photo).unwrap();
                assert!(
                    distance(back, [x, y]) < 1e-9,
                    "{:?} came back as {back:?}",
                    [x, y]
                );
                x += 2.0;
            }
            y += 2.0;
        }
    }
}

#[test]
fn the_crop_is_the_box_at_ten_pixels_a_millimetre() {
    let sizes: Vec<_> = [&STRIKEPRO_DRIVER, &STRIKEPRO_HYBRID, &STRIKEPRO_IRON]
        .iter()
        .map(|label| {
            let grid = CropGrid::new(label, CROP_PX_PER_MM);
            (label.id, grid.width, grid.height)
        })
        .collect();
    assert_eq!(
        sizes,
        [
            ("strikepro_driver", 905, 496),
            ("strikepro_hybrid", 735, 344),
            ("strikepro_iron", 730, 399),
        ]
    );
    assert_eq!(CROP_PX_PER_MM, 10.0);
}

#[test]
fn the_crop_grid_puts_the_cross_where_the_record_does_and_comes_back() {
    let grid = CropGrid::new(&STRIKEPRO_IRON, CROP_PX_PER_MM);
    // The cross is 36.6 mm from the box's left and 24.8 mm down from its top.
    let cross = grid.to_crop_px([0.0, 0.0]);
    assert!(distance(cross, [366.0, 248.0]) < 1e-9, "{cross:?}");
    // Row 0 is the top: up in the label is up in the crop.
    assert!(grid.to_label_mm([366.0, 0.0])[1] > grid.to_label_mm([366.0, 300.0])[1]);
    // The box's corners are the crop's, to the rounding of its size.
    let [top_left, _, bottom_right, _] = STRIKEPRO_IRON.box_corners_mm();
    assert!(distance(grid.to_crop_px(top_left), [0.0, 0.0]) < 1e-9);
    assert!(distance(grid.to_crop_px(bottom_right), [730.0, 399.0]) < 1e-9);
    for point in [[-12.34, 5.67], [30.0, -14.0], [0.05, 0.0]] {
        assert!(distance(grid.to_label_mm(grid.to_crop_px(point)), point) < 1e-12);
    }
}

#[test]
fn a_photo_at_the_crops_own_scale_is_cut_pixel_for_pixel() {
    // Corners that make the map a pure shift, one photo pixel to one crop pixel. Each crop pixel's
    // centre then lands on a photo pixel's centre, so a crop off by half a pixel (0.05 mm, inside
    // the dot test's 0.1 mm) differs here at once.
    let label = &STRIKEPRO_IRON;
    let grid = CropGrid::new(label, CROP_PX_PER_MM);
    let shift = [20.0, 30.0];
    let corners = label.box_corners_mm();
    let photo_corners = corners.map(|c| {
        let [x, y] = grid.to_crop_px(c);
        [x + shift[0], y + shift[1]]
    });
    let (width, height) = (800, 450);
    let mut rgb = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            rgb.extend([
                ((x * 7 + y * 13) % 256) as u8,
                (x % 256) as u8,
                (y % 256) as u8,
            ]);
        }
    }
    let photo = Frame { width, height, rgb };
    let h = Homography::from_corners(photo_corners, corners).unwrap();
    let crop = rectify(&photo, &h, label, CROP_PX_PER_MM);
    assert!(crop.outside.iter().all(|&outside| !outside));
    for row in 0..grid.height {
        let from = ((row + 30) * width + 20) * 3;
        let to = row * grid.width * 3;
        assert_eq!(
            &crop.frame.rgb[to..to + grid.width * 3],
            &photo.rgb[from..from + grid.width * 3],
            "row {row}"
        );
    }
}

#[test]
fn a_label_hanging_off_the_photo_is_marked_outside_where_the_photo_ends() {
    // Face-on, moved left until about a quarter of the driver label is off the photo's left edge.
    let scene = Scene {
        label: &STRIKEPRO_DRIVER,
        camera: Camera::new(0.0, 0.0, 0.0, 150.0, [-47.7, 0.0]),
        dot_mm: [20.0, 0.0],
    };
    let photo = draw(&scene);
    let crop = rectify(&photo, &homography_of(&scene), scene.label, CROP_PX_PER_MM);
    let mut marked = 0;
    for row in 0..crop.grid.height {
        for column in 0..crop.grid.width {
            let at = row * crop.grid.width + column;
            let mm = crop
                .grid
                .to_label_mm([column as f64 + 0.5, row as f64 + 0.5]);
            let [x, y] = scene.camera.project(mm);
            let in_photo =
                (0.0..PHOTO_WIDTH as f64).contains(&x) && (0.0..PHOTO_HEIGHT as f64).contains(&y);
            // A pixel whose sample lands within a millionth of a pixel of the photo's edge may
            // fall either side between the camera's arithmetic and the homography's.
            let near = |at: f64, edge: f64| (at - edge).abs() < 1e-6;
            let on_edge = near(x, 0.0)
                || near(y, 0.0)
                || near(x, PHOTO_WIDTH as f64)
                || near(y, PHOTO_HEIGHT as f64);
            if !on_edge {
                assert_eq!(crop.outside[at], !in_photo, "pixel ({column}, {row})");
            }
            if crop.outside[at] {
                marked += 1;
                assert_eq!(&crop.frame.rgb[at * 3..at * 3 + 3], &[0, 0, 0]);
            }
        }
    }
    let share = f64::from(marked) / (crop.grid.width * crop.grid.height) as f64;
    assert!((0.2..0.35).contains(&share), "{share}");
    // The dot is on the part still in the photo, and is still found.
    assert!(distance(ink_centroid_mm(&crop, scene.label), scene.dot_mm) <= RECOVERED_MM);
}

#[test]
fn a_photo_that_is_not_well_formed_is_all_outside_at_the_grids_size() {
    let scene = &scenes()[0];
    let broken = Frame {
        width: PHOTO_WIDTH,
        height: PHOTO_HEIGHT,
        rgb: vec![255; 10],
    };
    let crop = rectify(&broken, &homography_of(scene), scene.label, CROP_PX_PER_MM);
    assert_eq!((crop.frame.width, crop.frame.height), (905, 496));
    assert!(crop.frame.is_well_formed());
    assert!(crop.outside.iter().all(|&outside| outside));
}

/// A square label 20 mm across, its corners top-left, top-right, bottom-right, bottom-left.
const SQUARE_MM: [[f64; 2]; 4] = [[-10.0, 10.0], [10.0, 10.0], [10.0, -10.0], [-10.0, -10.0]];

/// The same four corners in a photo, seen a little off square, in the same order.
const PHOTO_QUAD: [[f64; 2]; 4] = [
    [100.0, 100.0],
    [300.0, 110.0],
    [310.0, 290.0],
    [90.0, 300.0],
];

#[test]
fn a_quad_seen_from_in_front_makes_a_homography() {
    assert!(Homography::from_corners(PHOTO_QUAD, SQUARE_MM).is_ok());
}

#[test]
fn four_corners_on_a_line_are_refused() {
    let line = [[0.0, 0.0], [100.0, 0.0], [200.0, 0.0], [300.0, 0.0]];
    assert_eq!(
        Homography::from_corners(line, SQUARE_MM),
        Err(Degenerate::Collinear)
    );
    // At a slant, and with the line on the label's side instead.
    let slant = [[0.0, 0.0], [100.0, 37.0], [200.0, 74.0], [300.0, 111.0]];
    assert_eq!(
        Homography::from_corners(slant, SQUARE_MM),
        Err(Degenerate::Collinear)
    );
    assert_eq!(
        Homography::from_corners(PHOTO_QUAD, line),
        Err(Degenerate::Collinear)
    );
}

#[test]
fn three_corners_on_a_line_are_refused() {
    let three = [
        [100.0, 100.0],
        [200.0, 150.0],
        [300.0, 200.0],
        [90.0, 300.0],
    ];
    assert_eq!(
        Homography::from_corners(three, SQUARE_MM),
        Err(Degenerate::Collinear)
    );
    // Off the line by a hair is still refused, and off it by a pixel, toward the outside so the
    // quad stays convex, is not: the tolerance is numerical, not a judgment of shape.
    let hair = [
        [100.0, 100.0],
        [200.0, 150.0 - 1e-9],
        [300.0, 200.0],
        [90.0, 300.0],
    ];
    assert_eq!(
        Homography::from_corners(hair, SQUARE_MM),
        Err(Degenerate::Collinear)
    );
    let pixel = [
        [100.0, 100.0],
        [200.0, 149.0],
        [300.0, 200.0],
        [90.0, 300.0],
    ];
    assert!(Homography::from_corners(pixel, SQUARE_MM).is_ok());
}

#[test]
fn a_repeated_corner_and_a_point_are_refused() {
    let repeated = [
        [100.0, 100.0],
        [300.0, 110.0],
        [300.0, 110.0],
        [90.0, 300.0],
    ];
    assert_eq!(
        Homography::from_corners(repeated, SQUARE_MM),
        Err(Degenerate::Collinear)
    );
    assert_eq!(
        Homography::from_corners([[5.0, 5.0]; 4], SQUARE_MM),
        Err(Degenerate::Collinear)
    );
}

#[test]
fn a_crossed_quad_is_refused_as_folded() {
    // Top-left, top-right, bottom-left, bottom-right: a bow-tie against the label's ring order.
    let crossed = [
        [100.0, 100.0],
        [300.0, 110.0],
        [90.0, 300.0],
        [310.0, 290.0],
    ];
    assert_eq!(
        Homography::from_corners(crossed, SQUARE_MM),
        Err(Degenerate::Folded)
    );
}

#[test]
fn a_corner_inside_the_other_three_is_refused_as_folded() {
    // (180, 160) is inside the triangle of the other three, whose long side is x + y = 400.
    let dented = [
        [100.0, 100.0],
        [300.0, 100.0],
        [180.0, 160.0],
        [100.0, 300.0],
    ];
    assert_eq!(
        Homography::from_corners(dented, SQUARE_MM),
        Err(Degenerate::Folded)
    );
}

#[test]
fn a_mirrored_quad_is_refused() {
    // The photo's corners listed right to left: the label as if seen from behind.
    let mirrored = [PHOTO_QUAD[1], PHOTO_QUAD[0], PHOTO_QUAD[3], PHOTO_QUAD[2]];
    assert_eq!(
        Homography::from_corners(mirrored, SQUARE_MM),
        Err(Degenerate::Mirrored)
    );
    // The photo's corners taken a quarter round instead: the label turned in the photo, which a
    // camera can see, whichever way round it is.
    for turn in 1..4 {
        let turned: [[f64; 2]; 4] = std::array::from_fn(|i| PHOTO_QUAD[(i + turn) % 4]);
        assert!(
            Homography::from_corners(turned, SQUARE_MM).is_ok(),
            "{turn}"
        );
    }
}

#[test]
fn a_corner_not_finite_is_refused() {
    let mut photo = PHOTO_QUAD;
    photo[2][1] = f64::NAN;
    assert_eq!(
        Homography::from_corners(photo, SQUARE_MM),
        Err(Degenerate::NotFinite)
    );
    let mut label = SQUARE_MM;
    label[0][0] = f64::INFINITY;
    assert_eq!(
        Homography::from_corners(PHOTO_QUAD, label),
        Err(Degenerate::NotFinite)
    );
}
