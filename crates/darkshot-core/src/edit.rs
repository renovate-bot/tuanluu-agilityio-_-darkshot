//! Chỉnh sửa hình đã vẽ: trúng hình, tay nắm, biến đổi, đổi kiểu.
//! Thuần logic, không phụ thuộc hệ điều hành.

use crate::annotate::{Pt, Shape, SizeLevel};
use crate::geometry::{Handle, Rect};
use crate::image::Image;
use crate::theme::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip {
    Move,
    Box(Handle),
    End(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gesture {
    pub index: usize,
    pub grip: Grip,
    pub start: Pt,
    pub original: Shape,
    pub original_bounds: Rect,
}

/// `p` trúng hình. `bounds`: hộp bao đo của hình (dùng cho Text). `tol` px.
pub fn hit_shape(shape: &Shape, bounds: Rect, p: Pt, tol: f32) -> bool {
    todo!()
}

pub fn topmost_hit(shapes: &[Shape], bounds: &[Rect], p: Pt, tol: f32) -> Option<usize> {
    todo!()
}

pub fn handles(shape: &Shape, bounds: Rect) -> Vec<(Grip, Pt)> {
    todo!()
}

pub fn grip_at(shape: &Shape, bounds: Rect, p: Pt, radius: f32) -> Option<Grip> {
    todo!()
}

pub fn frame(shape: &Shape, bounds: Rect) -> Option<Rect> {
    todo!()
}

pub fn transform(g: &Gesture, to: Pt, shift: bool, min_extent: f32, img: &Image) -> Shape {
    todo!()
}

pub fn restyle(shape: &Shape, level: Option<SizeLevel>, color: Option<Rgb>, dpi: u32) -> Shape {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotate::{
        blur_pixel_rect, redact_color, STEP_RADIUS_MAX_PX, STEP_RADIUS_MIN_PX, TEXT_SIZE_MAX_PX,
        TEXT_SIZE_MIN_PX,
    };

    const RED: Rgb = Rgb(255, 0, 0);
    const BLUE: Rgb = Rgb(0, 0, 255);
    const Z: Rect = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };

    fn p(x: f32, y: f32) -> Pt {
        Pt { x, y }
    }
    fn rc(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }
    /// 100×100, nửa trái đỏ thuần, nửa phải xanh thuần.
    fn img() -> Image {
        let mut bgra = Vec::new();
        for _y in 0..100 {
            for x in 0..100 {
                let c = if x < 50 { RED } else { BLUE };
                bgra.extend_from_slice(&[c.2, c.1, c.0, 255]);
            }
        }
        Image {
            width: 100,
            height: 100,
            bgra,
        }
    }
    fn rect(a: Pt, b: Pt) -> Shape {
        Shape::Rect {
            a,
            b,
            width: 4.0,
            color: RED,
        }
    }
    fn blur(a: Pt, b: Pt, color: Rgb) -> Shape {
        Shape::Blur { a, b, color }
    }
    fn gesture(shape: Shape, grip: Grip, start: Pt) -> Gesture {
        let ob = shape.bounds().unwrap_or(Z);
        Gesture {
            index: 0,
            grip,
            start,
            original: shape,
            original_bounds: ob,
        }
    }
    fn tf(shape: Shape, grip: Grip, to: Pt, shift: bool, min: f32) -> Shape {
        transform(&gesture(shape, grip, p(0.0, 0.0)), to, shift, min, &img())
    }

    #[test]
    fn hit_each_kind() {
        let pen = Shape::Pen {
            points: vec![p(10.0, 10.0), p(50.0, 10.0)],
            width: 4.0,
            color: RED,
        };
        assert!(hit_shape(&pen, Z, p(30.0, 13.0), 4.0));
        assert!(!hit_shape(&pen, Z, p(30.0, 30.0), 4.0));
        let marker = Shape::Marker {
            points: vec![p(10.0, 10.0)],
            width: 16.0,
            color: RED,
        };
        assert!(hit_shape(&marker, Z, p(15.0, 10.0), 4.0));
        assert!(!hit_shape(&marker, Z, p(40.0, 10.0), 4.0));
        let line = Shape::Line {
            a: p(0.0, 0.0),
            b: p(40.0, 0.0),
            width: 4.0,
            color: RED,
        };
        assert!(hit_shape(&line, Z, p(20.0, 5.0), 4.0));
        assert!(!hit_shape(&line, Z, p(20.0, 15.0), 4.0));
        let arrow = Shape::Arrow {
            a: p(0.0, 0.0),
            b: p(60.0, 0.0),
            width: 3.0,
            color: RED,
        };
        assert!(hit_shape(&arrow, Z, p(30.0, 2.0), 4.0));
        assert!(hit_shape(&arrow, Z, p(52.0, 5.0), 0.0));
        assert!(!hit_shape(&arrow, Z, p(30.0, 20.0), 4.0));
        let r = rect(p(10.0, 10.0), p(50.0, 50.0));
        assert!(hit_shape(&r, Z, p(30.0, 10.0), 4.0));
        assert!(!hit_shape(&r, Z, p(30.0, 30.0), 4.0));
        let e = Shape::Ellipse {
            a: p(0.0, 0.0),
            b: p(40.0, 20.0),
            width: 4.0,
            color: RED,
        };
        assert!(hit_shape(&e, Z, p(20.0, 0.0), 4.0));
        assert!(!hit_shape(&e, Z, p(20.0, 10.0), 4.0));
        let st = Shape::Step {
            center: p(50.0, 50.0),
            radius: 10.0,
            n: 1,
            color: RED,
        };
        assert!(hit_shape(&st, Z, p(58.0, 50.0), 4.0));
        assert!(!hit_shape(&st, Z, p(70.0, 50.0), 4.0));
        let b = blur(p(10.0, 10.0), p(30.0, 30.0), RED);
        assert!(hit_shape(&b, Z, p(20.0, 20.0), 4.0));
        assert!(!hit_shape(&b, Z, p(50.0, 50.0), 4.0));
        let t = Shape::Text {
            origin: p(10.0, 10.0),
            text: "x".into(),
            size: 20.0,
            color: RED,
        };
        assert!(hit_shape(&t, rc(10, 10, 40, 20), p(30.0, 20.0), 4.0));
        assert!(hit_shape(&t, rc(10, 10, 40, 20), p(52.0, 20.0), 4.0));
        assert!(!hit_shape(&t, rc(10, 10, 40, 20), p(80.0, 80.0), 4.0));
    }

    #[test]
    fn topmost_wins() {
        let shapes = vec![
            blur(p(0.0, 0.0), p(40.0, 40.0), RED),
            blur(p(20.0, 20.0), p(60.0, 60.0), RED),
        ];
        let bounds: Vec<Rect> = shapes.iter().filter_map(|s| s.bounds()).collect();
        assert_eq!(topmost_hit(&shapes, &bounds, p(30.0, 30.0), 2.0), Some(1));
        assert_eq!(topmost_hit(&shapes, &bounds, p(5.0, 5.0), 2.0), Some(0));
        assert_eq!(topmost_hit(&shapes, &bounds, p(100.0, 100.0), 2.0), None);
        assert_eq!(
            topmost_hit(&shapes, &bounds[..1], p(30.0, 30.0), 2.0),
            Some(0)
        );
        assert_eq!(topmost_hit(&shapes, &[], p(30.0, 30.0), 2.0), None);
    }

    #[test]
    fn handles_per_kind() {
        let line = Shape::Line {
            a: p(0.0, 0.0),
            b: p(9.0, 9.0),
            width: 3.0,
            color: RED,
        };
        assert_eq!(handles(&line, Z).len(), 2);
        let r = rect(p(30.0, 40.0), p(10.0, 20.0));
        let hs = handles(&r, Z);
        assert_eq!(hs.len(), 8);
        assert_eq!(hs[0], (Grip::Box(Handle::NW), p(10.0, 20.0)));
        assert_eq!(handles(&blur(p(0.0, 0.0), p(9.0, 9.0), RED), Z).len(), 8);
        let e = Shape::Ellipse {
            a: p(0.0, 0.0),
            b: p(9.0, 9.0),
            width: 3.0,
            color: RED,
        };
        assert_eq!(handles(&e, Z).len(), 8);
        let pen = Shape::Pen {
            points: vec![p(0.0, 0.0), p(9.0, 5.0)],
            width: 3.0,
            color: RED,
        };
        assert_eq!(handles(&pen, Z).len(), 4);
        let mk = Shape::Marker {
            points: vec![p(0.0, 0.0), p(9.0, 5.0)],
            width: 3.0,
            color: RED,
        };
        assert_eq!(handles(&mk, Z).len(), 4);
        let t = Shape::Text {
            origin: p(1.0, 1.0),
            text: "a".into(),
            size: 18.0,
            color: RED,
        };
        assert_eq!(handles(&t, rc(1, 1, 10, 20)).len(), 4);
        let st = Shape::Step {
            center: p(5.0, 5.0),
            radius: 4.0,
            n: 1,
            color: RED,
        };
        assert_eq!(handles(&st, Z).len(), 4);
    }

    #[test]
    fn grip_at_picks_nearest() {
        let r = rect(p(10.0, 10.0), p(50.0, 50.0));
        assert_eq!(
            grip_at(&r, Z, p(13.0, 10.0), 5.0),
            Some(Grip::Box(Handle::NW))
        );
        assert_eq!(grip_at(&r, Z, p(16.0, 10.0), 5.0), None);
        let line = Shape::Line {
            a: p(0.0, 0.0),
            b: p(10.0, 0.0),
            width: 3.0,
            color: RED,
        };
        assert_eq!(grip_at(&line, Z, p(9.0, 1.0), 5.0), Some(Grip::End(1)));
    }

    #[test]
    fn frame_rules() {
        let r = rect(p(10.0, 10.0), p(50.0, 50.0));
        assert_eq!(frame(&r, rc(10, 10, 40, 40)), Some(rc(8, 8, 44, 44)));
        let line = Shape::Line {
            a: p(0.0, 0.0),
            b: p(9.0, 9.0),
            width: 3.0,
            color: RED,
        };
        assert_eq!(frame(&line, rc(0, 0, 9, 9)), None);
    }

    #[test]
    fn move_preserves_style() {
        let pen = Shape::Pen {
            points: vec![p(1.0, 1.0), p(5.0, 5.0)],
            width: 3.0,
            color: BLUE,
        };
        let pen2 = Shape::Pen {
            points: vec![p(8.0, -2.0), p(12.0, 2.0)],
            width: 3.0,
            color: BLUE,
        };
        let line = Shape::Line {
            a: p(1.0, 1.0),
            b: p(5.0, 5.0),
            width: 3.0,
            color: BLUE,
        };
        let line2 = Shape::Line {
            a: p(8.0, -2.0),
            b: p(12.0, 2.0),
            width: 3.0,
            color: BLUE,
        };
        let arrow = Shape::Arrow {
            a: p(1.0, 1.0),
            b: p(5.0, 5.0),
            width: 3.0,
            color: BLUE,
        };
        let arrow2 = Shape::Arrow {
            a: p(8.0, -2.0),
            b: p(12.0, 2.0),
            width: 3.0,
            color: BLUE,
        };
        let text = Shape::Text {
            origin: p(1.0, 1.0),
            text: "hi".into(),
            size: 22.0,
            color: BLUE,
        };
        let text2 = Shape::Text {
            origin: p(8.0, -2.0),
            text: "hi".into(),
            size: 22.0,
            color: BLUE,
        };
        let step = Shape::Step {
            center: p(1.0, 1.0),
            radius: 9.0,
            n: 3,
            color: BLUE,
        };
        let step2 = Shape::Step {
            center: p(8.0, -2.0),
            radius: 9.0,
            n: 3,
            color: BLUE,
        };
        let mk = Shape::Marker {
            points: vec![p(1.0, 1.0)],
            width: 16.0,
            color: BLUE,
        };
        let mk2 = Shape::Marker {
            points: vec![p(8.0, -2.0)],
            width: 16.0,
            color: BLUE,
        };
        let el = Shape::Ellipse {
            a: p(1.0, 1.0),
            b: p(5.0, 5.0),
            width: 3.0,
            color: BLUE,
        };
        let el2 = Shape::Ellipse {
            a: p(8.0, -2.0),
            b: p(12.0, 2.0),
            width: 3.0,
            color: BLUE,
        };
        let rt = Shape::Rect {
            a: p(1.0, 1.0),
            b: p(5.0, 5.0),
            width: 3.0,
            color: BLUE,
        };
        let rt2 = Shape::Rect {
            a: p(8.0, -2.0),
            b: p(12.0, 2.0),
            width: 3.0,
            color: BLUE,
        };
        let bl = blur(p(1.0, 1.0), p(5.0, 5.0), RED);
        let bl2 = blur(p(8.0, -2.0), p(12.0, 2.0), RED);
        for (s, want) in [
            (pen, pen2),
            (line, line2),
            (arrow, arrow2),
            (text, text2),
            (step, step2),
            (mk, mk2),
            (el, el2),
            (rt, rt2),
            (bl, bl2),
        ] {
            let g = gesture(s, Grip::Move, p(10.0, 10.0));
            assert_eq!(transform(&g, p(17.0, 7.0), false, 2.0, &img()), want);
        }
    }

    #[test]
    fn rect_resize_flip_and_min() {
        let r = rect(p(10.0, 10.0), p(50.0, 50.0));
        match tf(r.clone(), Grip::Box(Handle::SE), p(5.0, 60.0), false, 2.0) {
            Shape::Rect { a, b, .. } => {
                assert_eq!((a, b), (p(5.0, 10.0), p(10.0, 60.0)));
            }
            o => panic!("{o:?}"),
        }
        match tf(r.clone(), Grip::Box(Handle::SE), p(10.5, 30.0), false, 6.0) {
            Shape::Rect { a, b, .. } => assert!(near(b.x - a.x, 6.0)),
            o => panic!("{o:?}"),
        }
        match tf(r, Grip::Box(Handle::SE), p(90.0, 60.0), true, 2.0) {
            Shape::Rect { a, b, .. } => {
                assert!(
                    near(b.x - a.x, 80.0) && near(b.y - a.y, 80.0),
                    "{a:?} {b:?}"
                );
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn line_end_shift() {
        let l = Shape::Line {
            a: p(0.0, 0.0),
            b: p(30.0, 0.0),
            width: 3.0,
            color: RED,
        };
        match tf(l.clone(), Grip::End(1), p(50.0, 3.0), true, 2.0) {
            Shape::Line { a, b, .. } => {
                assert_eq!(a, p(0.0, 0.0));
                assert!(b.y == 0.0 && b.x > 40.0);
            }
            o => panic!("{o:?}"),
        }
        assert_eq!(tf(l.clone(), Grip::End(1), p(1.0, 0.0), false, 2.0), l);
    }

    #[test]
    fn pen_scale_keeps_width() {
        let pen = Shape::Pen {
            points: vec![p(0.0, 0.0), p(5.0, 0.0), p(10.0, 0.0)],
            width: 3.0,
            color: RED,
        };
        match tf(pen, Grip::Box(Handle::SE), p(20.0, 40.0), false, 2.0) {
            Shape::Pen { points, width, .. } => {
                assert_eq!(width, 3.0);
                let xs: Vec<f32> = points.iter().map(|q| q.x).collect();
                assert!(xs.iter().zip([0.0, 10.0, 20.0]).all(|(a, b)| near(*a, b)));
                assert!(points.iter().all(|q| q.y == 0.0));
            }
            o => panic!("{o:?}"),
        }
    }

    fn text() -> Shape {
        Shape::Text {
            origin: p(10.0, 10.0),
            text: "ab".into(),
            size: 20.0,
            color: RED,
        }
    }

    fn text_gesture(to: Pt) -> Shape {
        let g = Gesture {
            index: 0,
            grip: Grip::Box(Handle::SE),
            start: p(50.0, 30.0),
            original: text(),
            original_bounds: rc(10, 10, 40, 20),
        };
        transform(&g, to, false, 2.0, &img())
    }

    #[test]
    fn text_resize_uniform() {
        match text_gesture(p(90.0, 50.0)) {
            Shape::Text { origin, size, .. } => {
                assert!(near(size, 40.0));
                assert_eq!(origin, p(10.0, 10.0));
            }
            o => panic!("{o:?}"),
        }
        match text_gesture(p(10.5, 10.2)) {
            Shape::Text { size, .. } => assert_eq!(size, TEXT_SIZE_MIN_PX),
            o => panic!("{o:?}"),
        }
        match text_gesture(p(9000.0, 5000.0)) {
            Shape::Text { size, .. } => assert_eq!(size, TEXT_SIZE_MAX_PX),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn step_resize_clamped() {
        let st = Shape::Step {
            center: p(50.0, 50.0),
            radius: 10.0,
            n: 2,
            color: RED,
        };
        let go = |to: Pt| tf(st.clone(), Grip::Box(Handle::SE), to, false, 2.0);
        match go(p(100.0, 100.0)) {
            Shape::Step {
                center, radius, n, ..
            } => {
                assert!(near(radius, 30.0) && n == 2);
                assert!(near(center.x, 70.0) && near(center.y, 70.0));
            }
            o => panic!("{o:?}"),
        }
        match go(p(41.0, 41.0)) {
            Shape::Step { radius, .. } => assert_eq!(radius, STEP_RADIUS_MIN_PX),
            o => panic!("{o:?}"),
        }
        match go(p(1000.0, 1000.0)) {
            Shape::Step { radius, .. } => assert_eq!(radius, STEP_RADIUS_MAX_PX),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn blur_move_recolors_from_original() {
        let b = blur(p(10.0, 10.0), p(30.0, 30.0), RED);
        let g = gesture(b.clone(), Grip::Move, p(20.0, 20.0));
        match transform(&g, p(80.0, 20.0), false, 2.0, &img()) {
            Shape::Blur { a, color, .. } => {
                assert!(near(a.x, 70.0));
                assert_eq!(color, BLUE);
            }
            o => panic!("{o:?}"),
        }
        assert_eq!(g.original, b);
    }

    #[test]
    fn blur_resize_flip_stays_covered() {
        let im = img();
        let b = blur(p(60.0, 10.0), p(80.0, 30.0), BLUE);
        let g = gesture(b, Grip::Box(Handle::E), p(80.0, 20.0));
        match transform(&g, p(20.0, 20.0), false, 2.0, &im) {
            Shape::Blur { a, b, color } => {
                assert!(a.x < b.x);
                let r = blur_pixel_rect(a, b);
                assert!(r.w > 0 && r.h > 0);
                assert_eq!(Some(color), redact_color(&im, r));
                assert_ne!(color, BLUE);
            }
            o => panic!("{o:?}"),
        }
        let b = blur(p(60.0, 0.0), p(80.0, 20.0), BLUE);
        let g = gesture(b, Grip::Box(Handle::S), p(70.0, 20.0));
        match transform(&g, p(70.0, -30.0), false, 2.0, &im) {
            Shape::Blur { color, .. } => assert_eq!(color, BLUE),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn restyle_rules() {
        let pen = Shape::Pen {
            points: vec![p(0.0, 0.0)],
            width: 3.0,
            color: RED,
        };
        match restyle(&pen, Some(SizeLevel::Large), None, 192) {
            Shape::Pen { width, color, .. } => assert!(near(width, 10.0) && color == RED),
            o => panic!("{o:?}"),
        }
        let mk = Shape::Marker {
            points: vec![p(0.0, 0.0)],
            width: 16.0,
            color: RED,
        };
        match restyle(&mk, Some(SizeLevel::Small), Some(BLUE), 96) {
            Shape::Marker { width, color, .. } => assert!(near(width, 10.0) && color == BLUE),
            o => panic!("{o:?}"),
        }
        match restyle(&text(), Some(SizeLevel::Huge), None, 96) {
            Shape::Text { size, origin, .. } => {
                assert!(near(size, 36.0));
                assert_eq!(origin, p(10.0, 10.0));
            }
            o => panic!("{o:?}"),
        }
        let st = Shape::Step {
            center: p(5.0, 5.0),
            radius: 13.0,
            n: 4,
            color: RED,
        };
        match restyle(&st, Some(SizeLevel::Small), None, 144) {
            Shape::Step {
                radius, n, center, ..
            } => assert!(near(radius, 15.0) && n == 4 && center == p(5.0, 5.0)),
            o => panic!("{o:?}"),
        }
        let b = blur(p(0.0, 0.0), p(9.0, 9.0), RED);
        assert_eq!(restyle(&b, Some(SizeLevel::Huge), Some(BLUE), 96), b);
        let l = Shape::Line {
            a: p(0.0, 0.0),
            b: p(9.0, 9.0),
            width: 3.0,
            color: RED,
        };
        match restyle(&l, None, Some(BLUE), 96) {
            Shape::Line { color, width, .. } => assert!(color == BLUE && width == 3.0),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn wrong_grip_returns_original() {
        let r = rect(p(10.0, 10.0), p(50.0, 50.0));
        assert_eq!(tf(r.clone(), Grip::End(0), p(0.0, 0.0), false, 2.0), r);
        // Số thực lạ không panic.
        let _ = tf(
            r,
            Grip::Box(Handle::SE),
            p(f32::NAN, f32::INFINITY),
            true,
            2.0,
        );
    }
}
