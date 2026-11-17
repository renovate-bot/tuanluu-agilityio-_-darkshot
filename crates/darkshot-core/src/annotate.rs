//! Mô hình đánh dấu trên ảnh chụp: hình, công cụ, bảng màu, hình học và mô hình soạn chữ.
//! Thuần logic, không phụ thuộc hệ điều hành; lớp Windows vẽ và nối chuột/bàn phím.

use crate::geometry::{Hit, Rect};
use crate::image::Image;
use crate::theme::{contrast_ratio, Rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Pen,
    Line,
    Arrow,
    Rect,
    Ellipse,
    Marker,
    Blur,
    Text,
    Step,
}

impl Tool {
    /// Thứ tự trên thanh công cụ.
    pub const ALL: [Tool; 10] = [
        Tool::Select,
        Tool::Pen,
        Tool::Line,
        Tool::Arrow,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Marker,
        Tool::Blur,
        Tool::Text,
        Tool::Step,
    ];

    /// Mã phím ảo chữ hoa: V, P, L, A, R, E, H, B, T, N.
    pub fn from_key(vk: u16) -> Option<Tool> {
        match vk {
            0x56 => Some(Tool::Select),
            0x46 => Some(Tool::Ellipse),
            0x4E => Some(Tool::Step),
            0x50 => Some(Tool::Pen),
            0x4C => Some(Tool::Line),
            0x41 => Some(Tool::Arrow),
            0x52 => Some(Tool::Rect),
            0x48 => Some(Tool::Marker),
            0x42 => Some(Tool::Blur),
            0x54 => Some(Tool::Text),
            _ => None,
        }
    }
}

/// Bảng 48 màu cơ bản của hộp chọn màu Windows, theo hàng từ trên xuống.
pub const PALETTE: [Rgb; 48] = [
    Rgb::hex(0xFF8080),
    Rgb::hex(0xFFFF80),
    Rgb::hex(0x80FF80),
    Rgb::hex(0x00FF80),
    Rgb::hex(0x80FFFF),
    Rgb::hex(0x0080FF),
    Rgb::hex(0xFF80C0),
    Rgb::hex(0xFF80FF),
    Rgb::hex(0xFF0000),
    Rgb::hex(0xFFFF00),
    Rgb::hex(0x80FF00),
    Rgb::hex(0x00FF40),
    Rgb::hex(0x00FFFF),
    Rgb::hex(0x0080C0),
    Rgb::hex(0x8080C0),
    Rgb::hex(0xFF00FF),
    Rgb::hex(0x804040),
    Rgb::hex(0xFF8040),
    Rgb::hex(0x00FF00),
    Rgb::hex(0x008080),
    Rgb::hex(0x004080),
    Rgb::hex(0x8080FF),
    Rgb::hex(0x800040),
    Rgb::hex(0xFF0080),
    Rgb::hex(0x800000),
    Rgb::hex(0xFF8000),
    Rgb::hex(0x008000),
    Rgb::hex(0x008040),
    Rgb::hex(0x0000FF),
    Rgb::hex(0x0000A0),
    Rgb::hex(0x800080),
    Rgb::hex(0x8000FF),
    Rgb::hex(0x400000),
    Rgb::hex(0x804000),
    Rgb::hex(0x004000),
    Rgb::hex(0x004040),
    Rgb::hex(0x000080),
    Rgb::hex(0x000040),
    Rgb::hex(0x400040),
    Rgb::hex(0x400080),
    Rgb::hex(0x000000),
    Rgb::hex(0x808000),
    Rgb::hex(0x808040),
    Rgb::hex(0x808080),
    Rgb::hex(0x408080),
    Rgb::hex(0xC0C0C0),
    Rgb::hex(0x400040),
    Rgb::hex(0xFFFFFF),
];
pub const PALETTE_COLS: usize = 8;
pub const DEFAULT_COLOR: Rgb = PALETTE[8];
pub const STROKE_DIP: f32 = 3.0;
pub const MARKER_DIP: f32 = 16.0;
pub const MARKER_ALPHA: u8 = 102;
pub const TEXT_DIP: f32 = 18.0;
pub const TEXT_OUTLINE_DIP: f32 = 2.0;
pub const ARROW_HEAD_FACTOR: f32 = 4.0;
pub const ARROW_HEAD_MIN_DIP: f32 = 12.0;
pub const ARROW_HALF_ANGLE_DEG: f32 = 28.0;
pub const MIN_DRAG_PX: f32 = 2.0;
/// Độ dày viền vòng số / bán kính.
pub const STEP_RING: f32 = 0.12;
/// Cạnh tối thiểu khi đổi cỡ.
pub const MIN_EXTENT_DIP: f32 = 6.0;
/// Dung sai trúng hình.
pub const HIT_TOL_DIP: f32 = 4.0;
pub const TEXT_SIZE_MIN_PX: f32 = 8.0;
pub const TEXT_SIZE_MAX_PX: f32 = 400.0;
pub const STEP_RADIUS_MIN_PX: f32 = 6.0;
pub const STEP_RADIUS_MAX_PX: f32 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SizeLevel {
    Small,
    #[default]
    Medium,
    Large,
    Huge,
}

impl SizeLevel {
    pub const ALL: [SizeLevel; 4] = [
        SizeLevel::Small,
        SizeLevel::Medium,
        SizeLevel::Large,
        SizeLevel::Huge,
    ];

    pub fn smaller(self) -> SizeLevel {
        match self {
            SizeLevel::Small | SizeLevel::Medium => SizeLevel::Small,
            SizeLevel::Large => SizeLevel::Medium,
            SizeLevel::Huge => SizeLevel::Large,
        }
    }

    pub fn larger(self) -> SizeLevel {
        match self {
            SizeLevel::Small => SizeLevel::Medium,
            SizeLevel::Medium => SizeLevel::Large,
            SizeLevel::Large | SizeLevel::Huge => SizeLevel::Huge,
        }
    }

    /// Pen, Line, Arrow, Rect, Ellipse.
    pub fn stroke_dip(self) -> f32 {
        match self {
            SizeLevel::Small => 2.0,
            SizeLevel::Medium => STROKE_DIP,
            SizeLevel::Large => 5.0,
            SizeLevel::Huge => 8.0,
        }
    }

    pub fn marker_dip(self) -> f32 {
        match self {
            SizeLevel::Small => 10.0,
            SizeLevel::Medium => MARKER_DIP,
            SizeLevel::Large => 24.0,
            SizeLevel::Huge => 32.0,
        }
    }

    pub fn text_dip(self) -> f32 {
        match self {
            SizeLevel::Small => 14.0,
            SizeLevel::Medium => TEXT_DIP,
            SizeLevel::Large => 26.0,
            SizeLevel::Huge => 36.0,
        }
    }

    /// Bán kính vòng số.
    pub fn step_dip(self) -> f32 {
        match self {
            SizeLevel::Small => 10.0,
            SizeLevel::Medium => 13.0,
            SizeLevel::Large => 17.0,
            SizeLevel::Huge => 22.0,
        }
    }
}

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
    /// Vùng che: tô kín `blur_pixel_rect(a, b)` bằng `color` (màu trung bình của ảnh chụp gốc).
    Blur { a: Pt, b: Pt, color: Rgb },
    /// `origin`: góc trên trái dòng đầu; '\n' xuống dòng.
    Text {
        origin: Pt,
        text: String,
        size: f32,
        color: Rgb,
    },
    Ellipse {
        a: Pt,
        b: Pt,
        width: f32,
        color: Rgb,
    },
    /// Vòng đánh số; `n` do `Annotations` đánh lại (1 + số Step đứng trước trong danh sách).
    Step {
        center: Pt,
        radius: f32,
        n: u32,
        color: Rgb,
    },
}

/// Hộp bao của các điểm, nới `pad` mỗi phía, làm tròn ra ngoài.
fn bounds_of(pts: &[Pt], pad: f32) -> Option<Rect> {
    let first = pts.first()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
    for p in pts {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    let (fx, fy) = ((x0 - pad).floor() as i32, (y0 - pad).floor() as i32);
    let (cx, cy) = ((x1 + pad).ceil() as i32, (y1 + pad).ceil() as i32);
    Some(Rect {
        x: fx,
        y: fy,
        w: cx.saturating_sub(fx),
        h: cy.saturating_sub(fy),
    })
}

/// Độ dài tối thiểu của đầu mũi tên (px) theo độ dày nét: độ dày luôn là `STROKE_DIP` đổi theo DPI,
/// nên tỉ lệ độ dày / `STROKE_DIP` chính là hệ số DPI.
pub fn arrow_min_head(width: f32) -> f32 {
    ARROW_HEAD_MIN_DIP * width / STROKE_DIP
}

impl Shape {
    /// Hộp bao (tọa độ ảnh, làm tròn ra ngoài) gồm nửa độ dày nét, đầu mũi tên, +1 px khử răng cưa.
    /// `None` với Text (cần đo chữ: lớp Windows tính) và với danh sách điểm rỗng.
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Shape::Pen { points, width, .. } => bounds_of(points, width / 2.0 + 1.0),
            // Đầu vuông và góc nhọn có thể nhô ra tới nửa độ dày nhân căn 2.
            Shape::Marker { points, width, .. } => {
                bounds_of(points, width / 2.0 * std::f32::consts::SQRT_2 + 1.0)
            }
            Shape::Line { a, b, width, .. } => bounds_of(&[*a, *b], width / 2.0 + 1.0),
            Shape::Rect { a, b, width, .. } => {
                bounds_of(&[*a, *b], width / 2.0 * std::f32::consts::SQRT_2 + 1.0)
            }
            Shape::Arrow { a, b, width, .. } => {
                let g = arrow_geometry(*a, *b, *width, arrow_min_head(*width));
                bounds_of(
                    &[*a, *b, g.shaft_end, g.head[1], g.head[2]],
                    width / 2.0 + 1.0,
                )
            }
            Shape::Blur { a, b, .. } => {
                let r = blur_pixel_rect(*a, *b);
                Some(Rect {
                    x: r.x.saturating_sub(1),
                    y: r.y.saturating_sub(1),
                    w: r.w.saturating_add(2),
                    h: r.h.saturating_add(2),
                })
            }
            Shape::Text { .. } => None,
            Shape::Ellipse { a, b, width, .. } => bounds_of(&[*a, *b], width / 2.0 + 1.0),
            Shape::Step { center, radius, .. } => {
                bounds_of(&[*center], radius + radius * STEP_RING / 2.0 + 1.0)
            }
        }
    }
}

/// Hình chữ nhật điểm ảnh của vùng che: min làm tròn xuống (floor), max làm tròn lên (ceil).
pub fn blur_pixel_rect(a: Pt, b: Pt) -> Rect {
    // `as i32` bão hòa (NaN thành 0), không panic.
    let (x0, y0) = (a.x.min(b.x).floor() as i32, a.y.min(b.y).floor() as i32);
    let (x1, y1) = (a.x.max(b.x).ceil() as i32, a.y.max(b.y).ceil() as i32);
    Rect {
        x: x0,
        y: y0,
        w: x1.saturating_sub(x0).max(0),
        h: y1.saturating_sub(y0).max(0),
    }
}

/// Số mẫu tối đa khi tính màu che.
pub const REDACT_MAX_SAMPLES: u64 = 65_536;

/// Bước lưới mẫu cho vùng w×h: số nguyên nhỏ nhất s ≥ 1 sao cho ceil(w/s)·ceil(h/s) ≤ REDACT_MAX_SAMPLES.
pub fn redact_step(w: i32, h: i32) -> i32 {
    let (w, h) = (w.max(0) as u64, h.max(0) as u64);
    let fits = |s: u64| w.div_ceil(s) * h.div_ceil(s) <= REDACT_MAX_SAMPLES;
    // Số mẫu không tăng khi s tăng; s = max(w, h, 1) cho tối đa 1 mẫu: tìm nhị phân.
    let (mut lo, mut hi) = (1u64, w.max(h).max(1));
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if fits(mid) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    i32::try_from(lo).unwrap_or(i32::MAX)
}

/// Trung bình cộng B, G, R (làm tròn gần nhất) của các điểm ảnh (x, y) với
/// x = r'.x + i·s, y = r'.y + j·s nằm trong r', trong đó r' = r ∩ ảnh và s = redact_step(r'.w, r'.h).
/// None khi r' rỗng hoặc ảnh không hợp lệ.
pub fn redact_color(img: &Image, r: Rect) -> Option<Rgb> {
    if !img.is_valid() {
        return None;
    }
    let x0 = i64::from(r.x).max(0);
    let y0 = i64::from(r.y).max(0);
    let x1 = (i64::from(r.x) + i64::from(r.w)).min(i64::from(img.width));
    let y1 = (i64::from(r.y) + i64::from(r.h)).min(i64::from(img.height));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    // Ảnh hợp lệ nên các cạnh vừa usize; vùng đã cắt vào ảnh.
    let (x0, y0, x1, y1) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);
    let s = redact_step(
        i32::try_from(x1 - x0).unwrap_or(i32::MAX),
        i32::try_from(y1 - y0).unwrap_or(i32::MAX),
    )
    .max(1) as usize;
    let stride = img.width as usize * 4;
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    for y in (y0..y1).step_by(s) {
        let row = img.bgra.get(y * stride..(y + 1) * stride)?;
        for x in (x0..x1).step_by(s) {
            let px = row.get(x * 4..x * 4 + 3)?;
            for (acc, v) in sum.iter_mut().zip(px) {
                *acc += u64::from(*v);
            }
            n += 1;
        }
    }
    if n == 0 {
        return None;
    }
    let avg = |v: u64| ((v + n / 2) / n).min(255) as u8;
    // BGRA: sum[0] = B, sum[1] = G, sum[2] = R.
    Some(Rgb(avg(sum[2]), avg(sum[1]), avg(sum[0])))
}

/// `dip * dpi / 96`.
pub fn dip_to_px(dip: f32, dpi: u32) -> f32 {
    dip * dpi as f32 / 96.0
}

/// Thêm điểm; bỏ qua nếu cách điểm cuối dưới 1 px.
pub fn add_point(points: &mut Vec<Pt>, p: Pt) {
    if let Some(last) = points.last() {
        let (dx, dy) = (p.x - last.x, p.y - last.y);
        if dx * dx + dy * dy < 1.0 {
            return;
        }
    }
    points.push(p);
}

/// Kéo `b` về hướng gần nhất trong 8 hướng (bội 45°), giữ độ dài hình chiếu (không âm).
pub fn constrain_45(a: Pt, b: Pt) -> Pt {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    if dx == 0.0 && dy == 0.0 {
        return b;
    }
    let k = (dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round() as i32;
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let (ux, uy) = match k.rem_euclid(8) {
        0 => (1.0, 0.0),
        1 => (s, s),
        2 => (0.0, 1.0),
        3 => (-s, s),
        4 => (-1.0, 0.0),
        5 => (-s, -s),
        6 => (0.0, -1.0),
        _ => (s, -s),
    };
    let len = (dx * ux + dy * uy).max(0.0);
    Pt {
        x: a.x + ux * len,
        y: a.y + uy * len,
    }
}

/// Cạnh = max(|dx|, |dy|), giữ dấu của từng hướng (0 coi là dương).
pub fn constrain_square(a: Pt, b: Pt) -> Pt {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let side = dx.abs().max(dy.abs());
    Pt {
        x: a.x + if dx < 0.0 { -side } else { side },
        y: a.y + if dy < 0.0 { -side } else { side },
    }
}

/// Line/Arrow: |ab| < MIN_DRAG_PX; Rect, Ellipse, Blur: |dx| hoặc |dy| < MIN_DRAG_PX; công cụ khác: false.
pub fn too_short(tool: Tool, a: Pt, b: Pt) -> bool {
    let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
    match tool {
        Tool::Line | Tool::Arrow => dx.hypot(dy) < MIN_DRAG_PX,
        Tool::Rect | Tool::Ellipse | Tool::Blur => dx < MIN_DRAG_PX || dy < MIN_DRAG_PX,
        _ => false,
    }
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
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = dx.hypot(dy);
    let (ux, uy) = if len > 0.0 {
        (dx / len, dy / len)
    } else {
        (1.0, 0.0)
    };
    let head_len = (ARROW_HEAD_FACTOR * width).max(min_head).min(len);
    let theta = ARROW_HALF_ANGLE_DEG.to_radians();
    let (cos, sin) = (theta.cos(), theta.sin());
    let (nx, ny) = (-uy, ux);
    let base = Pt {
        x: b.x - ux * head_len * cos,
        y: b.y - uy * head_len * cos,
    };
    let side = |sign: f32| Pt {
        x: base.x + nx * head_len * sin * sign,
        y: base.y + ny * head_len * sin * sign,
    };
    let shaft_end = if head_len >= len { a } else { base };
    ArrowGeom {
        shaft_end,
        head: [b, side(1.0), side(-1.0)],
    }
}

/// Đen hoặc trắng, cái có tỉ lệ tương phản với `c` cao hơn (hòa thì đen).
pub fn outline_color(c: Rgb) -> Rgb {
    let black = Rgb::hex(0x000000);
    let white = Rgb::hex(0xFFFFFF);
    if contrast_ratio(c, white) > contrast_ratio(c, black) {
        white
    } else {
        black
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressMode {
    Draw,
    Selection,
    Ignore,
    Pick,
}

/// Quyết định khi bấm chuột trong lớp phủ (ngoài thanh công cụ và bảng màu):
/// có công cụ: Handle -> Selection, Inside -> Draw, Outside -> Ignore;
/// công cụ Chọn: Inside, Outside -> Pick;
/// không công cụ: Outside khi `has_shapes` -> Ignore, còn lại -> Selection.
pub fn press_mode(tool: Option<Tool>, has_shapes: bool, hit: Hit) -> PressMode {
    match (tool, hit) {
        (Some(Tool::Select), Hit::Inside | Hit::Outside) => PressMode::Pick,
        (Some(_), Hit::Handle(_)) => PressMode::Selection,
        (Some(_), Hit::Inside) => PressMode::Draw,
        (Some(_), Hit::Outside) => PressMode::Ignore,
        (None, Hit::Outside) if has_shapes => PressMode::Ignore,
        (None, _) => PressMode::Selection,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// Hình mới nằm ở `index` (sau thêm, hoặc hoàn tác một lần xóa).
    Inserted { index: usize },
    /// Hình `shape` đã bị lấy khỏi `index`.
    Removed { index: usize, shape: Shape },
    /// Hình ở `index` đã được thay; `old` là hình trước đó.
    Replaced { index: usize, old: Shape },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    pub change: Change,
    /// Vị trí (sau thay đổi) các Step khác đổi `n`, tăng dần; không gồm `index` của `change`.
    pub renumbered: Vec<usize>,
}

#[derive(Clone, Debug)]
enum Edit {
    Add {
        index: usize,
        shape: Shape,
    },
    Remove {
        index: usize,
        shape: Shape,
    },
    Replace {
        index: usize,
        before: Shape,
        after: Shape,
    },
}

#[derive(Clone, Debug, Default)]
pub struct Annotations {
    shapes: Vec<Shape>,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}

impl Annotations {
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }

    /// Đánh lại `n` của mọi Step theo thứ tự (1, 2, 3...); trả các vị trí đổi `n`.
    fn renumber(&mut self) -> Vec<usize> {
        let mut changed = Vec::new();
        let mut next = 1u32;
        for (i, s) in self.shapes.iter_mut().enumerate() {
            if let Shape::Step { n, .. } = s {
                if *n != next {
                    *n = next;
                    changed.push(i);
                }
                next = next.saturating_add(1);
            }
        }
        changed
    }

    /// Đóng gói kết quả: `exclude` là vị trí của chính thay đổi (nếu hình còn đó).
    fn finish(&mut self, change: Change, exclude: Option<usize>) -> Applied {
        let mut renumbered = self.renumber();
        renumbered.retain(|i| Some(*i) != exclude);
        Applied { change, renumbered }
    }

    /// Thêm vào cuối, ghi sổ, xóa ngăn làm lại.
    pub fn add(&mut self, s: Shape) -> Applied {
        let index = self.shapes.len();
        self.shapes.push(s);
        self.renumber();
        if let Some(shape) = self.shapes.get(index) {
            self.undo.push(Edit::Add {
                index,
                shape: shape.clone(),
            });
        }
        self.redo.clear();
        self.finish(Change::Inserted { index }, Some(index))
    }

    /// None khi `index` ngoài danh sách.
    pub fn remove(&mut self, index: usize) -> Option<Applied> {
        if index >= self.shapes.len() {
            return None;
        }
        let shape = self.shapes.remove(index);
        self.undo.push(Edit::Remove {
            index,
            shape: shape.clone(),
        });
        self.redo.clear();
        Some(self.finish(Change::Removed { index, shape }, None))
    }

    /// None khi `index` ngoài danh sách hoặc `s` (sau khi đánh số theo vị trí) bằng hệt hình cũ.
    pub fn replace(&mut self, index: usize, mut s: Shape) -> Option<Applied> {
        let before = self.shapes.get(index)?.clone();
        if let Shape::Step { n, .. } = &mut s {
            let prior = self.shapes[..index]
                .iter()
                .filter(|x| matches!(x, Shape::Step { .. }))
                .count();
            *n = u32::try_from(prior).unwrap_or(u32::MAX).saturating_add(1);
        }
        if s == before {
            return None;
        }
        *self.shapes.get_mut(index)? = s.clone();
        self.undo.push(Edit::Replace {
            index,
            before: before.clone(),
            after: s,
        });
        self.redo.clear();
        Some(self.finish(Change::Replaced { index, old: before }, Some(index)))
    }

    /// Áp một mục sổ theo chiều thuận (`forward`) hoặc ngược; None khi vị trí không hợp lệ.
    fn apply(&mut self, edit: &Edit, forward: bool) -> Option<Applied> {
        match (edit, forward) {
            (Edit::Add { index, shape }, true) | (Edit::Remove { index, shape }, false) => {
                if *index > self.shapes.len() {
                    return None;
                }
                self.shapes.insert(*index, shape.clone());
                Some(self.finish(Change::Inserted { index: *index }, Some(*index)))
            }
            (Edit::Add { index, .. }, false) | (Edit::Remove { index, .. }, true) => {
                if *index >= self.shapes.len() {
                    return None;
                }
                let shape = self.shapes.remove(*index);
                Some(self.finish(
                    Change::Removed {
                        index: *index,
                        shape,
                    },
                    None,
                ))
            }
            (
                Edit::Replace {
                    index,
                    before,
                    after,
                },
                fwd,
            ) => {
                let (old, new) = if fwd {
                    (before, after)
                } else {
                    (after, before)
                };
                *self.shapes.get_mut(*index)? = new.clone();
                Some(self.finish(
                    Change::Replaced {
                        index: *index,
                        old: old.clone(),
                    },
                    Some(*index),
                ))
            }
        }
    }

    pub fn undo(&mut self) -> Option<Applied> {
        let edit = self.undo.pop()?;
        let applied = self.apply(&edit, false);
        self.redo.push(edit);
        applied
    }

    pub fn redo(&mut self) -> Option<Applied> {
        let edit = self.redo.pop()?;
        let applied = self.apply(&edit, true);
        self.undo.push(edit);
        applied
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Số mà một Step thêm vào cuối sẽ mang (1 + số Step hiện có).
    pub fn next_step_number(&self) -> u32 {
        let count = self
            .shapes
            .iter()
            .filter(|s| matches!(s, Shape::Step { .. }))
            .count();
        u32::try_from(count).unwrap_or(u32::MAX).saturating_add(1)
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    pub fn into_shapes(self) -> Vec<Shape> {
        self.shapes
    }
}

/// "\r\n" và "\r" thành "\n"; bỏ ký tự điều khiển khác ('\t' thành ' ').
pub fn sanitize_paste(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                out.push('\n');
            }
            '\n' => out.push('\n'),
            '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
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
        TextEdit {
            origin,
            chars: Vec::new(),
            caret: 0,
            color,
            size,
        }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Chỉ số đầu dòng chứa vị trí `i`.
    fn line_start(&self, i: usize) -> usize {
        let i = i.min(self.chars.len());
        self.chars[..i]
            .iter()
            .rposition(|c| *c == '\n')
            .map_or(0, |p| p + 1)
    }

    /// Chỉ số cuối dòng (vị trí '\n' kế tiếp hoặc hết chuỗi) tính từ `i`.
    fn line_end(&self, i: usize) -> usize {
        let i = i.min(self.chars.len());
        self.chars[i..]
            .iter()
            .position(|c| *c == '\n')
            .map_or(self.chars.len(), |p| i + p)
    }

    /// (dòng, cột), đếm từ 0.
    pub fn line_col(&self) -> (usize, usize) {
        let line = self.chars[..self.caret]
            .iter()
            .filter(|c| **c == '\n')
            .count();
        (line, self.caret - self.line_start(self.caret))
    }

    /// Chèn tại con trỏ, lọc như `sanitize_paste`.
    pub fn insert(&mut self, s: &str) {
        for c in sanitize_paste(s).chars() {
            self.chars.insert(self.caret, c);
            self.caret += 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.caret > 0 {
            self.caret -= 1;
            self.chars.remove(self.caret);
        }
    }

    pub fn delete(&mut self) {
        if self.caret < self.chars.len() {
            self.chars.remove(self.caret);
        }
    }

    pub fn left(&mut self) {
        self.caret = self.caret.saturating_sub(1);
    }

    pub fn right(&mut self) {
        if self.caret < self.chars.len() {
            self.caret += 1;
        }
    }

    /// Đầu dòng hiện tại.
    pub fn home(&mut self) {
        self.caret = self.line_start(self.caret);
    }

    /// Cuối dòng hiện tại.
    pub fn end(&mut self) {
        self.caret = self.line_end(self.caret);
    }

    /// Cùng cột, kẹp theo độ dài dòng đích; ở dòng đầu thì giữ nguyên.
    pub fn up(&mut self) {
        let start = self.line_start(self.caret);
        if start == 0 {
            return;
        }
        let col = self.caret - start;
        let prev_end = start - 1;
        let prev_start = self.line_start(prev_end);
        self.caret = prev_start + col.min(prev_end - prev_start);
    }

    /// Cùng cột, kẹp theo độ dài dòng đích; ở dòng cuối thì giữ nguyên.
    pub fn down(&mut self) {
        let end = self.line_end(self.caret);
        if end >= self.chars.len() {
            return;
        }
        let col = self.caret - self.line_start(self.caret);
        let next_start = end + 1;
        let next_end = self.line_end(next_start);
        self.caret = next_start + col.min(next_end - next_start);
    }

    pub fn set_color(&mut self, c: Rgb) {
        self.color = c;
    }

    /// `None` khi chữ chỉ có khoảng trắng hoặc xuống dòng.
    pub fn to_shape(&self) -> Option<Shape> {
        if self.chars.iter().all(|c| c.is_whitespace()) {
            return None;
        }
        Some(Shape::Text {
            origin: self.origin,
            text: self.text(),
            size: self.size,
            color: self.color,
        })
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
            0xFF8080, 0xFFFF80, 0x80FF80, 0x00FF80, 0x80FFFF, 0x0080FF, 0xFF80C0, 0xFF80FF,
            0xFF0000, 0xFFFF00, 0x80FF00, 0x00FF40, 0x00FFFF, 0x0080C0, 0x8080C0, 0xFF00FF,
            0x804040, 0xFF8040, 0x00FF00, 0x008080, 0x004080, 0x8080FF, 0x800040, 0xFF0080,
            0x800000, 0xFF8000, 0x008000, 0x008040, 0x0000FF, 0x0000A0, 0x800080, 0x8000FF,
            0x400000, 0x804000, 0x004000, 0x004040, 0x000080, 0x000040, 0x400040, 0x400080,
            0x000000, 0x808000, 0x808040, 0x808080, 0x408080, 0xC0C0C0, 0x400040, 0xFFFFFF,
        ];
        for (c, w) in PALETTE.iter().zip(want) {
            assert_eq!(*c, Rgb::hex(w));
        }
        assert_eq!(PALETTE.len(), 48);
        assert_eq!(PALETTE_COLS, 8);
        assert_eq!(DEFAULT_COLOR, PALETTE[8]);
        assert_eq!(DEFAULT_COLOR, Rgb::hex(0xFF0000));
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
        // Unikey xóa cả "êt" rồi gửi lại "ệt"
        e.backspace();
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

    #[test]
    fn blur_pixel_rect_rounds_outward() {
        let want = Rect {
            x: 1,
            y: 2,
            w: 5,
            h: 1,
        };
        assert_eq!(blur_pixel_rect(p(1.2, 2.7), p(5.1, 3.0)), want);
        assert_eq!(blur_pixel_rect(p(5.1, 3.0), p(1.2, 2.7)), want);
    }

    #[test]
    fn redact_step_caps_samples() {
        assert_eq!(redact_step(256, 256), 1);
        assert_eq!(redact_step(257, 256), 2);
        let n = |w: i32, h: i32, s: i32| {
            let c = |v: i32| (i64::from(v) + i64::from(s) - 1) / i64::from(s);
            (c(w) * c(h)) as u64
        };
        for (w, h) in [(7680, 2160), (1, 10_000_000), (3840, 2160)] {
            let s = redact_step(w, h);
            assert!(s >= 1);
            assert!(n(w, h, s) <= REDACT_MAX_SAMPLES, "{w}x{h} s={s}");
            if s > 1 {
                assert!(n(w, h, s - 1) > REDACT_MAX_SAMPLES, "{w}x{h} s={s}");
            }
        }
    }

    fn img_from(w: u32, h: u32, f: impl Fn(u32, u32) -> Rgb) -> Image {
        let mut bgra = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let c = f(x, y);
                bgra.extend_from_slice(&[c.2, c.1, c.0, 255]);
            }
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    #[test]
    fn redact_color_average() {
        let rc = |x, y, w, h| Rect { x, y, w, h };
        let black = Rgb(0, 0, 0);
        let white = Rgb(255, 255, 255);
        // 4×2, nửa trái đen, nửa phải trắng: 127,5 làm tròn lên.
        let half = img_from(4, 2, |x, _| if x < 2 { black } else { white });
        assert_eq!(
            redact_color(&half, rc(0, 0, 4, 2)),
            Some(Rgb(128, 128, 128))
        );
        assert_eq!(redact_color(&half, rc(0, 0, 2, 2)), Some(black));
        let flat = img_from(5, 3, |_, _| Rgb::hex(0x336699));
        assert_eq!(
            redact_color(&flat, rc(0, 0, 5, 3)),
            Some(Rgb::hex(0x336699))
        );
        let grad = img_from(6, 5, |x, y| {
            Rgb((x * 40) as u8, (y * 50) as u8, (x * y) as u8)
        });
        // Tràn ra ngoài ảnh: bằng vùng đã cắt.
        assert!(redact_color(&grad, rc(0, 2, 4, 3)).is_some());
        assert_eq!(
            redact_color(&grad, rc(-3, 2, 7, 10)),
            redact_color(&grad, rc(0, 2, 4, 3))
        );
        assert_eq!(
            redact_color(&half, rc(-10, -10, 100, 100)),
            Some(Rgb(128, 128, 128))
        );
        // Hoàn toàn ngoài ảnh, hoặc rỗng.
        assert_eq!(redact_color(&grad, rc(6, 0, 3, 3)), None);
        assert_eq!(redact_color(&grad, rc(-5, -5, 5, 5)), None);
        assert_eq!(redact_color(&grad, rc(0, 0, 0, 3)), None);
        // 1×1: đúng điểm ảnh đó.
        assert_eq!(redact_color(&grad, rc(3, 4, 1, 1)), Some(Rgb(120, 200, 12)));
        let bad = Image {
            width: 2,
            height: 2,
            bgra: vec![0; 3],
        };
        assert_eq!(redact_color(&bad, rc(0, 0, 2, 2)), None);
    }

    #[test]
    fn tool_blur_key_order() {
        assert_eq!(Tool::from_key(0x42), Some(Tool::Blur));
        assert_eq!(Tool::ALL[7], Tool::Blur);
        assert_eq!(Tool::ALL[8], Tool::Text);
        for (a, b) in [
            (p(0.0, 0.0), p(10.0, 1.0)),
            (p(0.0, 0.0), p(10.0, 5.0)),
            (p(0.0, 0.0), p(1.0, 10.0)),
            (p(3.0, 3.0), p(3.0, 3.0)),
            (p(0.0, 0.0), p(-2.0, -2.0)),
        ] {
            assert_eq!(too_short(Tool::Blur, a, b), too_short(Tool::Rect, a, b));
        }
        assert!(too_short(Tool::Blur, p(0.0, 0.0), p(10.0, 1.0)));
        assert!(!too_short(Tool::Blur, p(0.0, 0.0), p(10.0, 5.0)));
    }

    #[test]
    fn blur_bounds() {
        let s = Shape::Blur {
            a: p(10.5, 10.5),
            b: p(20.2, 30.0),
            color: DEFAULT_COLOR,
        };
        assert_eq!(
            s.bounds(),
            Some(Rect {
                x: 9,
                y: 9,
                w: 13,
                h: 22
            })
        );
    }

    #[test]
    fn tool_keys_and_order() {
        assert_eq!(
            Tool::ALL,
            [
                Tool::Select,
                Tool::Pen,
                Tool::Line,
                Tool::Arrow,
                Tool::Rect,
                Tool::Ellipse,
                Tool::Marker,
                Tool::Blur,
                Tool::Text,
                Tool::Step
            ]
        );
        assert_eq!(Tool::from_key(0x56), Some(Tool::Select));
        assert_eq!(Tool::from_key(0x46), Some(Tool::Ellipse));
        assert_eq!(Tool::from_key(0x4E), Some(Tool::Step));
        assert_eq!(Tool::from_key(0x50), Some(Tool::Pen));
        assert_eq!(Tool::from_key(0x4C), Some(Tool::Line));
        assert_eq!(Tool::from_key(0x41), Some(Tool::Arrow));
        assert_eq!(Tool::from_key(0x52), Some(Tool::Rect));
        assert_eq!(Tool::from_key(0x48), Some(Tool::Marker));
        assert_eq!(Tool::from_key(0x42), Some(Tool::Blur));
        assert_eq!(Tool::from_key(0x54), Some(Tool::Text));
    }

    #[test]
    fn size_level_values() {
        use SizeLevel::*;
        assert_eq!(SizeLevel::ALL, [Small, Medium, Large, Huge]);
        assert_eq!(SizeLevel::default(), Medium);
        let stroke = SizeLevel::ALL.map(|l| l.stroke_dip());
        assert_eq!(stroke, [2.0, 3.0, 5.0, 8.0]);
        let marker = SizeLevel::ALL.map(|l| l.marker_dip());
        assert_eq!(marker, [10.0, 16.0, 24.0, 32.0]);
        let text = SizeLevel::ALL.map(|l| l.text_dip());
        assert_eq!(text, [14.0, 18.0, 26.0, 36.0]);
        let step = SizeLevel::ALL.map(|l| l.step_dip());
        assert_eq!(step, [10.0, 13.0, 17.0, 22.0]);
        assert_eq!(Medium.stroke_dip(), STROKE_DIP);
        assert_eq!(Medium.marker_dip(), MARKER_DIP);
        assert_eq!(Medium.text_dip(), TEXT_DIP);
        assert_eq!(Small.smaller(), Small);
        assert_eq!(Huge.larger(), Huge);
        assert_eq!(Small.larger(), Medium);
        assert_eq!(Medium.larger(), Large);
        assert_eq!(Large.larger(), Huge);
        assert_eq!(Huge.smaller(), Large);
        assert_eq!(Medium.smaller(), Small);
    }

    #[test]
    fn ellipse_and_step_bounds() {
        let e = Shape::Ellipse {
            a: p(10.0, 10.0),
            b: p(30.0, 20.0),
            width: 4.0,
            color: DEFAULT_COLOR,
        };
        assert_eq!(
            e.bounds(),
            Some(Rect {
                x: 7,
                y: 7,
                w: 26,
                h: 16
            })
        );
        let s = Shape::Step {
            center: p(50.0, 50.0),
            radius: 10.0,
            n: 1,
            color: DEFAULT_COLOR,
        };
        assert_eq!(
            s.bounds(),
            Some(Rect {
                x: 38,
                y: 38,
                w: 24,
                h: 24
            })
        );
    }

    #[test]
    fn too_short_ellipse_like_rect() {
        for (a, b) in [
            (p(0.0, 0.0), p(10.0, 1.0)),
            (p(0.0, 0.0), p(10.0, 5.0)),
            (p(0.0, 0.0), p(1.0, 10.0)),
            (p(3.0, 3.0), p(3.0, 3.0)),
        ] {
            assert_eq!(too_short(Tool::Ellipse, a, b), too_short(Tool::Rect, a, b));
        }
        assert!(too_short(Tool::Ellipse, p(0.0, 0.0), p(10.0, 1.0)));
        assert!(!too_short(Tool::Step, p(0.0, 0.0), p(0.0, 0.0)));
    }

    #[test]
    fn press_mode_select() {
        use crate::geometry::Handle;
        for has in [false, true] {
            assert_eq!(
                press_mode(Some(Tool::Select), has, Hit::Inside),
                PressMode::Pick
            );
            assert_eq!(
                press_mode(Some(Tool::Select), has, Hit::Outside),
                PressMode::Pick
            );
            assert_eq!(
                press_mode(Some(Tool::Select), has, Hit::Handle(Handle::N)),
                PressMode::Selection
            );
        }
    }

    fn line(x: f32) -> Shape {
        Shape::Line {
            a: p(x, 0.0),
            b: p(x, 9.0),
            width: 3.0,
            color: DEFAULT_COLOR,
        }
    }
    fn step(x: f32, n: u32) -> Shape {
        Shape::Step {
            center: p(x, 5.0),
            radius: 10.0,
            n,
            color: DEFAULT_COLOR,
        }
    }
    fn step_ns(a: &Annotations) -> Vec<u32> {
        a.shapes()
            .iter()
            .filter_map(|s| match s {
                Shape::Step { n, .. } => Some(*n),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn add_undo_redo_multi() {
        let (sa, sb, sc) = (line(1.0), line(2.0), line(3.0));
        let mut a = Annotations::default();
        assert!(!a.can_undo() && !a.can_redo());
        a.add(sa.clone());
        a.add(sb.clone());
        a.add(sc.clone());
        assert!(a.can_undo() && !a.can_redo());
        let rm = |index, shape| {
            Some(Applied {
                change: Change::Removed { index, shape },
                renumbered: vec![],
            })
        };
        assert_eq!(a.undo(), rm(2, sc.clone()));
        assert!(a.can_undo() && a.can_redo());
        assert_eq!(a.undo(), rm(1, sb.clone()));
        assert_eq!(a.undo(), rm(0, sa.clone()));
        assert!(!a.can_undo() && a.can_redo());
        assert_eq!(a.undo(), None);
        assert!(a.shapes().is_empty());
        for i in 0..3 {
            let r = a.redo().unwrap();
            assert_eq!(r.change, Change::Inserted { index: i });
            assert!(a.can_undo());
            assert_eq!(a.can_redo(), i < 2);
        }
        assert_eq!(a.redo(), None);
        assert_eq!(a.shapes(), &[sa, sb, sc]);
    }

    #[test]
    fn new_edit_clears_redo() {
        let (sa, sb, sc) = (line(1.0), line(2.0), line(3.0));
        let mut a = Annotations::default();
        a.add(sa.clone());
        a.add(sb);
        a.undo();
        assert!(a.can_redo());
        a.add(sc.clone());
        assert!(!a.can_redo());
        assert_eq!(a.shapes(), &[sa, sc]);
    }

    #[test]
    fn remove_middle_then_undo_restores_position() {
        let (sa, sb, sc) = (line(1.0), line(2.0), line(3.0));
        let mut a = Annotations::default();
        a.add(sa.clone());
        a.add(sb.clone());
        a.add(sc.clone());
        let r = a.remove(1).unwrap();
        assert_eq!(
            r.change,
            Change::Removed {
                index: 1,
                shape: sb.clone()
            }
        );
        assert_eq!(a.shapes(), &[sa.clone(), sc.clone()]);
        let r = a.undo().unwrap();
        assert_eq!(r.change, Change::Inserted { index: 1 });
        assert_eq!(a.shapes(), &[sa.clone(), sb, sc.clone()]);
        a.redo().unwrap();
        assert_eq!(a.shapes(), &[sa, sc]);
    }

    #[test]
    fn replace_and_undo() {
        let (sa, a2) = (line(1.0), line(7.0));
        let mut a = Annotations::default();
        a.add(sa.clone());
        let r = a.replace(0, a2.clone()).unwrap();
        assert_eq!(
            r.change,
            Change::Replaced {
                index: 0,
                old: sa.clone()
            }
        );
        assert_eq!(a.shapes()[0], a2);
        let r = a.undo().unwrap();
        assert_eq!(
            r.change,
            Change::Replaced {
                index: 0,
                old: a2.clone()
            }
        );
        assert_eq!(a.shapes()[0], sa);
        a.redo().unwrap();
        assert_eq!(a.shapes()[0], a2);
    }

    #[test]
    fn replace_identical_is_not_recorded() {
        let sa = line(1.0);
        let mut a = Annotations::default();
        a.add(sa.clone());
        assert_eq!(a.replace(0, sa.clone()), None);
        assert_eq!(a.replace(5, line(2.0)), None);
        assert_eq!(a.remove(5), None);
        // Chỉ còn mục add: một lần hoàn tác là hết.
        assert!(a.undo().is_some());
        assert!(!a.can_undo());
    }

    #[test]
    fn steps_renumber() {
        let mut a = Annotations::default();
        assert_eq!(a.next_step_number(), 1);
        for x in [1.0, 2.0, 3.0] {
            a.add(step(x, 0));
        }
        assert_eq!(step_ns(&a), vec![1, 2, 3]);
        assert_eq!(a.next_step_number(), 4);
        let r = a.remove(0).unwrap();
        assert_eq!(step_ns(&a), vec![1, 2]);
        assert_eq!(r.renumbered, vec![0, 1]);
        let r = a.undo().unwrap();
        assert_eq!(r.change, Change::Inserted { index: 0 });
        assert_eq!(step_ns(&a), vec![1, 2, 3]);
        assert_eq!(r.renumbered, vec![1, 2]);
        // Hình khác chèn giữa không đổi số Step.
        let mut b = Annotations::default();
        b.add(step(1.0, 0));
        let r = b.add(line(1.0));
        assert!(r.renumbered.is_empty());
        b.add(step(2.0, 0));
        assert_eq!(step_ns(&b), vec![1, 2]);
        assert_eq!(b.next_step_number(), 3);
    }

    #[test]
    fn replace_step_keeps_number() {
        let mut a = Annotations::default();
        a.add(step(1.0, 0));
        a.add(step(2.0, 0));
        let r = a.replace(1, step(9.0, 99)).unwrap();
        assert_eq!(
            r.change,
            Change::Replaced {
                index: 1,
                old: step(2.0, 2)
            }
        );
        assert_eq!(a.shapes()[1], step(9.0, 2));
        assert_eq!(a.replace(1, step(9.0, 77)), None);
    }

    #[test]
    fn history_memory_is_per_shape() {
        let big = |w: f32| Shape::Pen {
            points: (0..10_000).map(|i| p(i as f32, 0.0)).collect(),
            width: w,
            color: DEFAULT_COLOR,
        };
        let first = big(1.0);
        let mut a = Annotations::default();
        a.add(first.clone());
        for i in 0..100 {
            a.replace(0, big(2.0 + i as f32)).unwrap();
        }
        assert_eq!(a.shapes().len(), 1);
        for _ in 0..100 {
            assert!(a.undo().is_some());
        }
        assert_eq!(a.shapes()[0], first);
    }
}
