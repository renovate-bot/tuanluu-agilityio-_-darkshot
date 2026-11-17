//! Mô hình đánh dấu trên ảnh chụp: hình, công cụ, bảng màu, hình học và mô hình soạn chữ.
//! Thuần logic, không phụ thuộc hệ điều hành; lớp Windows vẽ và nối chuột/bàn phím.

use crate::geometry::{Hit, Rect};
use crate::theme::{contrast_ratio, Rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Line,
    Arrow,
    Rect,
    Marker,
    Text,
}

impl Tool {
    /// Thứ tự trên thanh công cụ.
    pub const ALL: [Tool; 6] = [
        Tool::Pen,
        Tool::Line,
        Tool::Arrow,
        Tool::Rect,
        Tool::Marker,
        Tool::Text,
    ];

    /// Mã phím ảo chữ hoa: P, L, A, R, H, T.
    pub fn from_key(vk: u16) -> Option<Tool> {
        todo!()
    }
}

pub const PALETTE: [Rgb; 8] = [
    Rgb::hex(0xFF3B30),
    Rgb::hex(0xFF9500),
    Rgb::hex(0xFFCC00),
    Rgb::hex(0x34C759),
    Rgb::hex(0x0A84FF),
    Rgb::hex(0xAF52DE),
    Rgb::hex(0xFFFFFF),
    Rgb::hex(0x000000),
];
pub const DEFAULT_COLOR: Rgb = PALETTE[0];
pub const STROKE_DIP: f32 = 3.0;
pub const MARKER_DIP: f32 = 16.0;
pub const MARKER_ALPHA: u8 = 102;
pub const TEXT_DIP: f32 = 18.0;
pub const TEXT_OUTLINE_DIP: f32 = 2.0;
pub const ARROW_HEAD_FACTOR: f32 = 4.0;
pub const ARROW_HEAD_MIN_DIP: f32 = 12.0;
pub const ARROW_HALF_ANGLE_DEG: f32 = 28.0;
pub const MIN_DRAG_PX: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Pen {
        points: Vec<Pt>,
        width: f32,
        color: Rgb,
    },
    Line {
        a: Pt,
        b: Pt,
        width: f32,
        color: Rgb,
    },
    Arrow {
        a: Pt,
        b: Pt,
        width: f32,
        color: Rgb,
    },
    Rect {
        a: Pt,
        b: Pt,
        width: f32,
        color: Rgb,
    },
    Marker {
        points: Vec<Pt>,
        width: f32,
        color: Rgb,
    },
    /// `origin`: góc trên trái dòng đầu; '\n' xuống dòng.
    Text {
        origin: Pt,
        text: String,
        size: f32,
        color: Rgb,
    },
}

/// Hộp bao của các điểm, nới `pad` mỗi phía, làm tròn ra ngoài.
fn bounds_of(pts: &[Pt], pad: f32) -> Option<Rect> {
    todo!()
}

/// Độ dài tối thiểu của đầu mũi tên (px) theo độ dày nét: độ dày luôn là `STROKE_DIP` đổi theo DPI,
/// nên tỉ lệ độ dày / `STROKE_DIP` chính là hệ số DPI.
pub fn arrow_min_head(width: f32) -> f32 {
    todo!()
}

impl Shape {
    /// Hộp bao (tọa độ ảnh, làm tròn ra ngoài) gồm nửa độ dày nét, đầu mũi tên, +1 px khử răng cưa.
    /// `None` với Text (cần đo chữ: lớp Windows tính) và với danh sách điểm rỗng.
    pub fn bounds(&self) -> Option<Rect> {
        todo!()
    }
}

/// `dip * dpi / 96`.
pub fn dip_to_px(dip: f32, dpi: u32) -> f32 {
    todo!()
}

/// Thêm điểm; bỏ qua nếu cách điểm cuối dưới 1 px.
pub fn add_point(points: &mut Vec<Pt>, p: Pt) {
    todo!()
}

/// Kéo `b` về hướng gần nhất trong 8 hướng (bội 45°), giữ độ dài hình chiếu (không âm).
pub fn constrain_45(a: Pt, b: Pt) -> Pt {
    todo!()
}

/// Cạnh = max(|dx|, |dy|), giữ dấu của từng hướng (0 coi là dương).
pub fn constrain_square(a: Pt, b: Pt) -> Pt {
    todo!()
}

/// Line/Arrow: |ab| < MIN_DRAG_PX; Rect: |dx| hoặc |dy| < MIN_DRAG_PX; công cụ khác: false.
pub fn too_short(tool: Tool, a: Pt, b: Pt) -> bool {
    todo!()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowGeom {
    pub shaft_end: Pt,
    /// `head[0]` là mũi (b).
    pub head: [Pt; 3],
}

/// Đầu dài `max(4 × width, min_head)` (cạnh bên của tam giác); nếu dài hơn cả mũi tên thì
/// đầu = |ab| và `shaft_end = a`. Thân dừng ở gốc đầu mũi tên.
pub fn arrow_geometry(a: Pt, b: Pt, width: f32, min_head: f32) -> ArrowGeom {
    todo!()
}

/// Đen hoặc trắng, cái có tỉ lệ tương phản với `c` cao hơn (hòa thì đen).
pub fn outline_color(c: Rgb) -> Rgb {
    todo!()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressMode {
    Draw,
    Selection,
    Ignore,
}

/// Quyết định khi bấm chuột trong lớp phủ (ngoài thanh công cụ và bảng màu):
/// có công cụ: Handle -> Selection, Inside -> Draw, Outside -> Ignore;
/// không công cụ: Outside khi `has_shapes` -> Ignore, còn lại -> Selection.
pub fn press_mode(tool: Option<Tool>, has_shapes: bool, hit: Hit) -> PressMode {
    todo!()
}

#[derive(Clone, Debug, Default)]
pub struct Annotations {
    shapes: Vec<Shape>,
}

impl Annotations {
    pub fn shapes(&self) -> &[Shape] {
        todo!()
    }

    pub fn push(&mut self, s: Shape) {
        todo!()
    }

    /// Bỏ hình cuối.
    pub fn undo(&mut self) -> Option<Shape> {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }

    pub fn into_shapes(self) -> Vec<Shape> {
        todo!()
    }
}

/// "\r\n" và "\r" thành "\n"; bỏ ký tự điều khiển khác ('\t' thành ' ').
pub fn sanitize_paste(s: &str) -> String {
    todo!()
}

/// Mô hình soạn chữ: con trỏ theo chỉ số ký tự (char), dòng tách bởi '\n'.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEdit {
    origin: Pt,
    chars: Vec<char>,
    caret: usize,
    color: Rgb,
    size: f32,
}

impl TextEdit {
    pub fn new(origin: Pt, size: f32, color: Rgb) -> TextEdit {
        todo!()
    }

    pub fn text(&self) -> String {
        todo!()
    }

    pub fn caret(&self) -> usize {
        todo!()
    }

    /// Chỉ số đầu dòng chứa vị trí `i`.
    fn line_start(&self, i: usize) -> usize {
        todo!()
    }

    /// Chỉ số cuối dòng (vị trí '\n' kế tiếp hoặc hết chuỗi) tính từ `i`.
    fn line_end(&self, i: usize) -> usize {
        todo!()
    }

    /// (dòng, cột), đếm từ 0.
    pub fn line_col(&self) -> (usize, usize) {
        todo!()
    }

    /// Chèn tại con trỏ, lọc như `sanitize_paste`.
    pub fn insert(&mut self, s: &str) {
        todo!()
    }

    pub fn backspace(&mut self) {
        todo!()
    }

    pub fn delete(&mut self) {
        todo!()
    }

    pub fn left(&mut self) {
        todo!()
    }

    pub fn right(&mut self) {
        todo!()
    }

    /// Đầu dòng hiện tại.
    pub fn home(&mut self) {
        todo!()
    }

    /// Cuối dòng hiện tại.
    pub fn end(&mut self) {
        todo!()
    }

    /// Cùng cột, kẹp theo độ dài dòng đích; ở dòng đầu thì giữ nguyên.
    pub fn up(&mut self) {
        todo!()
    }

    /// Cùng cột, kẹp theo độ dài dòng đích; ở dòng cuối thì giữ nguyên.
    pub fn down(&mut self) {
        todo!()
    }

    pub fn set_color(&mut self, c: Rgb) {
        todo!()
    }

    /// `None` khi chữ chỉ có khoảng trắng hoặc xuống dòng.
    pub fn to_shape(&self) -> Option<Shape> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast_ratio;

    fn p(x: f32, y: f32) -> Pt {
        Pt { x, y }
    }
    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }
    fn dist(a: Pt, b: Pt) -> f32 {
        ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
    }
    fn contains(r: Rect, x: f32, y: f32) -> bool {
        x >= r.x as f32 && x <= r.right() as f32 && y >= r.y as f32 && y <= r.bottom() as f32
    }

    #[test]
    fn palette_and_constants_match_plan() {
        let want = [
            0xFF3B30, 0xFF9500, 0xFFCC00, 0x34C759, 0x0A84FF, 0xAF52DE, 0xFFFFFF, 0x000000,
        ];
        for (c, w) in PALETTE.iter().zip(want) {
            assert_eq!(*c, Rgb::hex(w));
        }
        assert_eq!(PALETTE.len(), 8);
        assert_eq!(DEFAULT_COLOR, PALETTE[0]);
        assert_eq!(STROKE_DIP, 3.0);
        assert_eq!(MARKER_DIP, 16.0);
        assert_eq!(MARKER_ALPHA, 102);
        assert_eq!(TEXT_DIP, 18.0);
        assert_eq!(TEXT_OUTLINE_DIP, 2.0);
        assert_eq!(ARROW_HEAD_FACTOR, 4.0);
        assert_eq!(ARROW_HEAD_MIN_DIP, 12.0);
        assert_eq!(ARROW_HALF_ANGLE_DEG, 28.0);
        assert_eq!(MIN_DRAG_PX, 2.0);
    }

    #[test]
    fn tool_from_key() {
        assert_eq!(Tool::from_key(0x50), Some(Tool::Pen));
        assert_eq!(Tool::from_key(0x4C), Some(Tool::Line));
        assert_eq!(Tool::from_key(0x41), Some(Tool::Arrow));
        assert_eq!(Tool::from_key(0x52), Some(Tool::Rect));
        assert_eq!(Tool::from_key(0x48), Some(Tool::Marker));
        assert_eq!(Tool::from_key(0x54), Some(Tool::Text));
        assert_eq!(Tool::from_key(0x5A), None);
        assert_eq!(Tool::from_key(0), None);
        assert_eq!(
            Tool::ALL,
            [
                Tool::Pen,
                Tool::Line,
                Tool::Arrow,
                Tool::Rect,
                Tool::Marker,
                Tool::Text
            ]
        );
    }

    #[test]
    fn dip_to_px_scales() {
        assert_eq!(dip_to_px(3.0, 96), 3.0);
        assert_eq!(dip_to_px(3.0, 144), 4.5);
        assert_eq!(dip_to_px(16.0, 192), 32.0);
    }

    #[test]
    fn add_point_skips_sub_pixel_moves() {
        let mut v = Vec::new();
        add_point(&mut v, p(5.0, 5.0));
        assert_eq!(v.len(), 1);
        add_point(&mut v, p(5.5, 5.0));
        assert_eq!(v.len(), 1);
        add_point(&mut v, p(6.0, 5.0));
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn constrain_45_snaps() {
        assert_eq!(constrain_45(p(0.0, 0.0), p(10.0, 1.0)), p(10.0, 0.0));
        let d = constrain_45(p(0.0, 0.0), p(10.0, 9.0));
        assert!(d.x > 0.0 && (d.x.abs() - d.y.abs()).abs() < 1e-4);
        assert_eq!(constrain_45(p(0.0, 0.0), p(-1.0, -10.0)), p(0.0, -10.0));
        // điểm trùng nhau không panic
        assert_eq!(constrain_45(p(3.0, 3.0), p(3.0, 3.0)), p(3.0, 3.0));
    }

    #[test]
    fn constrain_square_keeps_sign() {
        // dx = -4, dy = 7 -> cạnh 7
        assert_eq!(constrain_square(p(5.0, 5.0), p(1.0, 12.0)), p(-2.0, 12.0));
        assert_eq!(constrain_square(p(0.0, 0.0), p(3.0, -9.0)), p(9.0, -9.0));
    }

    #[test]
    fn too_short_rules() {
        assert!(too_short(Tool::Line, p(0.0, 0.0), p(1.0, 1.0)));
        assert!(!too_short(Tool::Line, p(0.0, 0.0), p(2.0, 0.0)));
        assert!(too_short(Tool::Arrow, p(0.0, 0.0), p(1.0, 0.0)));
        assert!(too_short(Tool::Rect, p(0.0, 0.0), p(10.0, 1.0)));
        assert!(!too_short(Tool::Rect, p(0.0, 0.0), p(10.0, 5.0)));
        assert!(!too_short(Tool::Pen, p(0.0, 0.0), p(0.0, 0.0)));
        assert!(!too_short(Tool::Marker, p(0.0, 0.0), p(0.0, 0.0)));
        assert!(!too_short(Tool::Text, p(0.0, 0.0), p(0.0, 0.0)));
    }

    #[test]
    fn arrow_geometry_normal_and_short() {
        let a = p(0.0, 0.0);
        let b = p(100.0, 0.0);
        let g = arrow_geometry(a, b, 3.0, 12.0);
        assert_eq!(g.head[0], b);
        let cos = ARROW_HALF_ANGLE_DEG.to_radians().cos();
        assert!(near(b.x - g.shaft_end.x, 12.0 * cos));
        assert!(near(g.shaft_end.y, 0.0));
        assert!(near(dist(g.head[0], g.head[1]), 12.0));
        assert!(near(dist(g.head[0], g.head[2]), 12.0));
        let s = arrow_geometry(a, p(5.0, 0.0), 3.0, 12.0);
        assert_eq!(s.shaft_end, a);
        assert!(near(dist(s.head[0], s.head[1]), 5.0));
        // độ dày lớn: 4 x 5 = 20 > min_head
        let w = arrow_geometry(a, b, 5.0, 12.0);
        assert!(near(dist(w.head[0], w.head[1]), 20.0));
        // dài 0 không panic
        let z = arrow_geometry(a, a, 3.0, 12.0);
        assert_eq!(z.shaft_end, a);
    }

    #[test]
    fn outline_color_picks_contrast() {
        let black = Rgb::hex(0);
        let white = Rgb::hex(0xFFFFFF);
        assert_eq!(outline_color(Rgb::hex(0xFF3B30)), black);
        assert_eq!(outline_color(black), white);
        assert_eq!(outline_color(white), black);
        let blue = Rgb::hex(0x0A84FF);
        let want = if contrast_ratio(blue, white) > contrast_ratio(blue, black) {
            white
        } else {
            black
        };
        assert_eq!(outline_color(blue), want);
    }

    #[test]
    fn press_mode_table() {
        use crate::geometry::Handle;
        let tools = [None, Some(Tool::Pen), Some(Tool::Text)];
        for tool in tools {
            for has in [false, true] {
                for hit in [Hit::Outside, Hit::Inside, Hit::Handle(Handle::SE)] {
                    let want = match (tool, hit) {
                        (Some(_), Hit::Handle(_)) => PressMode::Selection,
                        (Some(_), Hit::Inside) => PressMode::Draw,
                        (Some(_), Hit::Outside) => PressMode::Ignore,
                        (None, Hit::Outside) if has => PressMode::Ignore,
                        (None, _) => PressMode::Selection,
                    };
                    assert_eq!(press_mode(tool, has, hit), want, "{tool:?} {has} {hit:?}");
                }
            }
        }
        assert_eq!(press_mode(None, true, Hit::Outside), PressMode::Ignore);
        assert_eq!(press_mode(None, false, Hit::Outside), PressMode::Selection);
        assert_eq!(
            press_mode(
                Some(Tool::Pen),
                false,
                Hit::Handle(crate::geometry::Handle::SE)
            ),
            PressMode::Selection
        );
        assert_eq!(
            press_mode(Some(Tool::Pen), true, Hit::Inside),
            PressMode::Draw
        );
        assert_eq!(
            press_mode(Some(Tool::Text), false, Hit::Outside),
            PressMode::Ignore
        );
    }

    #[test]
    fn bounds_include_width_and_head() {
        let c = DEFAULT_COLOR;
        let line = Shape::Line {
            a: p(10.0, 10.0),
            b: p(20.0, 10.0),
            width: 4.0,
            color: c,
        };
        let r = line.bounds().unwrap();
        assert!(r.x <= 8 && r.y <= 8 && r.right() >= 22 && r.bottom() >= 12);
        assert!(r.x >= 6 && r.y >= 6 && r.right() <= 24 && r.bottom() <= 14);

        let (a, b) = (p(0.0, 0.0), p(60.0, 20.0));
        let arrow = Shape::Arrow {
            a,
            b,
            width: 3.0,
            color: c,
        };
        let r = arrow.bounds().unwrap();
        let g = arrow_geometry(a, b, 3.0, ARROW_HEAD_MIN_DIP);
        for v in g.head {
            assert!(contains(r, v.x, v.y), "{v:?} {r:?}");
        }

        let text = Shape::Text {
            origin: p(1.0, 1.0),
            text: "x".into(),
            size: 18.0,
            color: c,
        };
        assert_eq!(text.bounds(), None);

        let pen = Shape::Pen {
            points: vec![p(50.0, 50.0)],
            width: 3.0,
            color: c,
        };
        let r = pen.bounds().unwrap();
        assert!(contains(r, 48.5, 48.5) && contains(r, 51.5, 51.5));
        assert!(r.w <= 8 && r.h <= 8);
        let empty = Shape::Pen {
            points: vec![],
            width: 3.0,
            color: c,
        };
        assert_eq!(empty.bounds(), None);
    }

    #[test]
    fn annotations_undo() {
        let mut a = Annotations::default();
        assert!(a.is_empty());
        assert_eq!(a.undo(), None);
        let s1 = Shape::Line {
            a: p(0.0, 0.0),
            b: p(5.0, 5.0),
            width: 3.0,
            color: DEFAULT_COLOR,
        };
        let s2 = Shape::Rect {
            a: p(0.0, 0.0),
            b: p(5.0, 5.0),
            width: 3.0,
            color: DEFAULT_COLOR,
        };
        a.push(s1.clone());
        a.push(s2.clone());
        assert_eq!(a.shapes().len(), 2);
        assert_eq!(a.undo(), Some(s2));
        assert_eq!(a.shapes().len(), 1);
        assert_eq!(a.into_shapes(), vec![s1]);
    }

    fn edit() -> TextEdit {
        TextEdit::new(p(0.0, 0.0), 18.0, DEFAULT_COLOR)
    }

    #[test]
    fn text_edit_basic() {
        let mut e = edit();
        e.insert("ab");
        e.insert("\n");
        e.insert("c");
        assert_eq!(e.text(), "ab\nc");
        assert_eq!(e.line_col(), (1, 1));
        e.up();
        assert_eq!(e.line_col(), (0, 1));
        e.end();
        assert_eq!(e.line_col(), (0, 2));
        e.down();
        assert_eq!(e.line_col(), (1, 1));
        e.home();
        assert_eq!(e.line_col(), (1, 0));
        e.backspace();
        assert_eq!(e.text(), "abc");
        assert_eq!(e.line_col(), (0, 2));
        // biên: đầu và cuối không panic
        let mut e = edit();
        e.backspace();
        e.delete();
        e.left();
        e.up();
        e.down();
        e.insert("x");
        e.right();
        e.delete();
        assert_eq!(e.text(), "x");
        assert_eq!(e.caret(), 1);
        e.left();
        e.delete();
        assert_eq!(e.text(), "");
    }

    #[test]
    fn text_edit_unikey_sequence() {
        let mut e = edit();
        e.insert("Vie");
        e.backspace();
        e.insert("ê");
        e.insert("t");
        e.backspace();
        e.insert("ệ");
        e.insert("t");
        assert_eq!(e.text(), "Việt");
        assert_eq!(e.caret(), 4);
    }

    #[test]
    fn text_edit_to_shape() {
        let mut e = TextEdit::new(p(3.0, 4.0), 24.0, DEFAULT_COLOR);
        assert_eq!(e.to_shape(), None);
        e.insert(" \n ");
        assert_eq!(e.to_shape(), None);
        let mut e = TextEdit::new(p(3.0, 4.0), 24.0, DEFAULT_COLOR);
        e.insert("Hi");
        assert_eq!(
            e.to_shape(),
            Some(Shape::Text {
                origin: p(3.0, 4.0),
                text: "Hi".into(),
                size: 24.0,
                color: DEFAULT_COLOR
            })
        );
        e.set_color(PALETTE[4]);
        match e.to_shape() {
            Some(Shape::Text { color, .. }) => assert_eq!(color, PALETTE[4]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn sanitize_paste_normalizes_newlines() {
        assert_eq!(sanitize_paste("a\r\nb\rc\td\u{0007}"), "a\nb\nc d");
        assert_eq!(sanitize_paste(""), "");
    }
}
