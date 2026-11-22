//! Bỏ đường kẻ bảng trên bản sao một dải ảnh trước lượt OCR thứ hai (Task 136).
//!
//! Đường kẻ = dải ngang (hay dọc) dày 1–3 px, dài ≥ `MIN_LEN` px, mà mỗi điểm khác hẳn (độ sáng)
//! cả điểm ngay trên dải VÀ điểm ngay dưới dải (đường dọc: trái và phải), còn hai điểm hai bên
//! gần nhau. Luật dựa trên tương phản hai bên, không ngưỡng "tối tuyệt đối": chạy đúng cả nền sáng
//! kẻ tối lẫn nền tối kẻ sáng, và không xóa nền. Điểm đường kẻ được thay bằng trung bình màu hai
//! bên. Bản sao chỉ dùng để quét, không bao giờ xuất. Không ghi log, không lưu gì.

use crate::image::Image;

/// Độ dài tối thiểu của một đường kẻ (px), tính cả các khe nhỏ.
pub const MIN_LEN: usize = 40;
/// Độ dày tối đa của một đường kẻ (px). Nét chữ đậm/lớn dày hơn được giữ.
pub const MAX_THICK: usize = 3;
/// Khe dài nhất (px) cho phép giữa hai đoạn của cùng một đường (chân chữ g/y/p/q cắt qua,
/// chỗ giao với đường kẻ chiều kia). Điểm trong khe được giữ nguyên.
const MAX_GAP: usize = 8;
/// Điểm đường kẻ phải chiếm ít nhất 4/5 chiều dài (loại hàng đỉnh chữ `zzzz`, `TTTT`).
const MIN_FILL_NUM: usize = 4;
const MIN_FILL_DEN: usize = 5;
/// Chênh độ sáng tối thiểu giữa điểm đường kẻ và mỗi bên (0..255).
const MIN_CONTRAST: i32 = 32;
/// Chênh độ sáng tối đa giữa hai bên của đường kẻ.
const SIDE_TOL: i32 = 16;

/// Bản sao `bgra` (`width×height`, BGRA, từ trên xuống) đã bỏ đường kẻ bảng; `None` khi không thấy
/// đường kẻ nào (không cần lượt quét thứ hai) hoặc dữ liệu không hợp lệ. O(w·h); cấp phát lớn
/// nhất là một bản sao dải, chỉ khi có đường kẻ.
pub fn strip_rules(bgra: &[u8], width: u32, height: u32) -> Option<Image> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || w.checked_mul(h)?.checked_mul(4)? != bgra.len() {
        return None;
    }
    let mut buf = Buf {
        src: bgra,
        copy: None,
        w,
        h,
    };
    horizontal(&mut buf);
    vertical(&mut buf);
    buf.copy.map(|bgra| Image {
        width,
        height,
        bgra,
    })
}

/// Ảnh đang xử lý: đọc từ bản sao nếu đã có, không thì từ ảnh gốc; ghi lần đầu thì mới sao chép.
struct Buf<'a> {
    src: &'a [u8],
    copy: Option<Vec<u8>>,
    w: usize,
    h: usize,
}

impl Buf<'_> {
    fn data(&self) -> &[u8] {
        self.copy.as_deref().unwrap_or(self.src)
    }

    fn px(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.w + x) * 4;
        let d = self.data();
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    }

    fn lum(&self, x: usize, y: usize) -> i32 {
        let p = self.px(x, y);
        (i32::from(p[2]) * 77 + i32::from(p[1]) * 150 + i32::from(p[0]) * 29) >> 8
    }

    /// Độ sáng cả hàng `y` vào `out` (đọc theo thứ tự bộ nhớ).
    fn lum_row(&self, y: usize, out: &mut Vec<i32>) {
        let row = &self.data()[y * self.w * 4..(y + 1) * self.w * 4];
        out.clear();
        out.extend(
            row.as_chunks::<4>().0.iter().map(|p| {
                (i32::from(p[2]) * 77 + i32::from(p[1]) * 150 + i32::from(p[0]) * 29) >> 8
            }),
        );
    }

    fn put(&mut self, x: usize, y: usize, v: [u8; 4]) {
        let i = (y * self.w + x) * 4;
        let src = self.src;
        let d = self.copy.get_or_insert_with(|| src.to_vec());
        d[i..i + 4].copy_from_slice(&v);
    }
}

/// Trung bình từng kênh của hai điểm.
fn mid(a: [u8; 4], b: [u8; 4]) -> [u8; 4] {
    std::array::from_fn(|c| (u16::from(a[c]) + u16::from(b[c])).div_ceil(2) as u8)
}

/// `n` điểm `p(k)` (k = 0..n) là một dải đường kẻ giữa hai bên `a`, `b` (độ sáng; bên nào thiếu
/// vì ở mép ảnh thì dùng bên kia).
fn is_band(a: Option<i32>, b: Option<i32>, n: usize, p: impl Fn(usize) -> i32) -> bool {
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        (Some(a), None) => (a, a),
        (None, Some(b)) => (b, b),
        (None, None) => return false,
    };
    if (a - b).abs() > SIDE_TOL {
        return false;
    }
    (0..n).all(|k| {
        let v = p(k);
        let (da, db) = (v - a, v - b);
        da.abs() >= MIN_CONTRAST && db.abs() >= MIN_CONTRAST && (da > 0) == (db > 0)
    })
}

/// Một đường đang dựng dọc theo chiều chạy: điểm đầu, điểm cuối, số điểm đường kẻ.
#[derive(Clone, Copy, Default)]
struct Run {
    start: usize,
    last: usize,
    hits: usize,
    on: bool,
}

impl Run {
    /// Thêm điểm đường kẻ ở `i`. Trả đường cũ cần đóng nếu khe quá dài.
    fn hit(&mut self, i: usize) -> Option<Run> {
        let mut closed = None;
        if self.on && i - self.last - 1 > MAX_GAP {
            closed = Some(*self);
            self.on = false;
        }
        if !self.on {
            *self = Run {
                start: i,
                last: i,
                hits: 1,
                on: true,
            };
        } else {
            self.last = i;
            self.hits += 1;
        }
        closed
    }

    /// Đường đủ dài và đủ kín để bỏ.
    fn is_rule(&self) -> bool {
        let span = self.last - self.start + 1;
        self.on && span >= MIN_LEN && self.hits * MIN_FILL_DEN >= span * MIN_FILL_NUM
    }
}

/// Độ sáng của các hàng `y0 - 1 ..= y0 + MAX_THICK` (vòng `RING` hàng, hàng `r` ở ô `r % RING`).
const RING: usize = MAX_THICK + 2;

/// Dải ngang dày `t` bắt đầu ở hàng `y0` (cao `h`), tại cột `x`: là đường kẻ?
fn h_band(ring: &[Vec<i32>], h: usize, x: usize, y0: usize, t: usize) -> bool {
    let l = |y: usize| ring[y % RING][x];
    let above = y0.checked_sub(1).map(l);
    let below = (y0 + t < h).then(|| l(y0 + t));
    is_band(above, below, t, |k| l(y0 + k))
}

/// Dải dọc dày `t` bắt đầu ở cột `x0` trên một hàng có độ sáng `row`: là đường kẻ?
fn v_band(row: &[i32], x0: usize, t: usize) -> bool {
    let left = x0.checked_sub(1).map(|x| row[x]);
    let right = row.get(x0 + t).copied();
    is_band(left, right, t, |k| row[x0 + k])
}

/// Màu thay: trung bình hai bên (bên ở mép ảnh thiếu thì lấy bên kia).
fn fill(a: Option<[u8; 4]>, b: Option<[u8; 4]>) -> [u8; 4] {
    match (a, b) {
        (Some(a), Some(b)) => mid(a, b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => [0; 4],
    }
}

fn clear_h(b: &mut Buf, ring: &[Vec<i32>], r: Run, y0: usize, t: usize) {
    if !r.is_rule() {
        return;
    }
    for x in r.start..=r.last {
        if !h_band(ring, b.h, x, y0, t) {
            continue;
        }
        let a = y0.checked_sub(1).map(|y| b.px(x, y));
        let c = (y0 + t < b.h).then(|| b.px(x, y0 + t));
        let v = fill(a, c);
        for k in 0..t {
            b.put(x, y0 + k, v);
        }
    }
}

/// Đóng một đường dọc: dò lại từng hàng của nó (độ sáng đọc lại từ ảnh) rồi thay.
fn clear_v(b: &mut Buf, r: Run, x0: usize, t: usize) {
    if !r.is_rule() {
        return;
    }
    for y in r.start..=r.last {
        let lum = |x: usize| b.lum(x, y);
        let left = x0.checked_sub(1).map(lum);
        let right = (x0 + t < b.w).then(|| lum(x0 + t));
        if !is_band(left, right, t, |k| lum(x0 + k)) {
            continue;
        }
        let a = x0.checked_sub(1).map(|x| b.px(x, y));
        let c = (x0 + t < b.w).then(|| b.px(x0 + t, y));
        let v = fill(a, c);
        for k in 0..t {
            b.put(x0 + k, y, v);
        }
    }
}

/// Đường ngang: mỗi hàng đầu `y0`, chạy dọc hàng, một đường đang dựng cho mỗi độ dày. Độ sáng
/// đọc từ vòng `RING` hàng (dựng một lần mỗi hàng, từ ảnh lúc đó).
fn horizontal(b: &mut Buf) {
    if b.w < MIN_LEN {
        return;
    }
    let mut ring: Vec<Vec<i32>> = (0..RING).map(|_| Vec::with_capacity(b.w)).collect();
    for y in 0..(MAX_THICK + 1).min(b.h) {
        b.lum_row(y, &mut ring[y % RING]);
    }
    for y0 in 0..b.h {
        let next = y0 + MAX_THICK;
        if y0 > 0 && next < b.h {
            b.lum_row(next, &mut ring[next % RING]);
        }
        let tmax = MAX_THICK.min(b.h - y0);
        let mut runs = [Run::default(); MAX_THICK];
        // Lọc nhanh: điểm đầu dải phải tương phản với hàng trên (hàng dưới khi ở mép trên).
        let side = match (y0.checked_sub(1), y0 + 1 < b.h) {
            (Some(y), _) => y,
            (None, true) => y0 + 1,
            (None, false) => continue,
        };
        let (cur, near) = (&ring[y0 % RING], &ring[side % RING]);
        for x in 0..b.w {
            if (cur[x] - near[x]).abs() < MIN_CONTRAST {
                continue;
            }
            for t in 1..=tmax {
                if h_band(&ring, b.h, x, y0, t) {
                    if let Some(old) = runs[t - 1].hit(x) {
                        clear_h(b, &ring, old, y0, t);
                    }
                }
            }
        }
        for t in 1..=tmax {
            clear_h(b, &ring, runs[t - 1], y0, t);
        }
    }
}

/// Đường dọc: chạy theo hàng (đọc ảnh theo thứ tự bộ nhớ), giữ một đường đang dựng mỗi cột và độ dày.
fn vertical(b: &mut Buf) {
    if b.h < MIN_LEN || b.w < 2 {
        return;
    }
    let mut runs = vec![Run::default(); b.w * MAX_THICK];
    let mut row = Vec::with_capacity(b.w);
    for y in 0..b.h {
        b.lum_row(y, &mut row);
        for x0 in 0..b.w {
            // Lọc nhanh: điểm đầu dải phải tương phản với điểm bên trái (bên phải khi ở mép trái).
            let near = if x0 > 0 { row[x0 - 1] } else { row[1] };
            if (row[x0] - near).abs() < MIN_CONTRAST {
                continue;
            }
            for t in 1..=MAX_THICK.min(b.w - x0) {
                if v_band(&row, x0, t) {
                    let slot = x0 * MAX_THICK + t - 1;
                    if let Some(old) = runs[slot].hit(y) {
                        clear_v(b, old, x0, t);
                    }
                }
            }
        }
    }
    for x0 in 0..b.w {
        for t in 1..=MAX_THICK.min(b.w - x0) {
            let r = runs[x0 * MAX_THICK + t - 1];
            clear_v(b, r, x0, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const BLACK: [u8; 4] = [0, 0, 0, 255];

    struct Canvas {
        w: usize,
        h: usize,
        px: Vec<u8>,
    }

    impl Canvas {
        fn new(w: usize, h: usize, bg: [u8; 4]) -> Canvas {
            Canvas {
                w,
                h,
                px: bg.repeat(w * h),
            }
        }
        fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, c: [u8; 4]) {
            for yy in y..y + h {
                for xx in x..x + w {
                    let i = (yy * self.w + xx) * 4;
                    self.px[i..i + 4].copy_from_slice(&c);
                }
            }
        }
        fn at(img: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
            let i = (y * w + x) * 4;
            [img[i], img[i + 1], img[i + 2], img[i + 3]]
        }
        fn strip(&self) -> Option<Image> {
            strip_rules(&self.px, self.w as u32, self.h as u32)
        }
        /// Số điểm khác nhau giữa ảnh gốc và `img`.
        fn changed(&self, img: &Image) -> usize {
            self.px
                .as_chunks::<4>()
                .0
                .iter()
                .zip(img.bgra.as_chunks::<4>().0)
                .filter(|(a, b)| a != b)
                .count()
        }
    }

    fn gray(v: u8) -> [u8; 4] {
        [v, v, v, 255]
    }

    /// Mọi điểm của `img` bằng `c`.
    fn all(img: &Image, c: [u8; 4]) -> bool {
        img.bgra.as_chunks::<4>().0.iter().all(|p| *p == c)
    }

    #[test]
    fn light_background_dark_rules_of_1_2_3_px_are_removed() {
        for t in 1..=3 {
            let mut c = Canvas::new(300, 60, WHITE);
            c.rect(10, 20, 250, t, gray(60));
            let out = c.strip().expect("rule found");
            assert_eq!((out.width, out.height), (300, 60));
            assert!(all(&out, WHITE), "thickness {t}");
            assert_eq!(c.changed(&out), 250 * t);
        }
    }

    #[test]
    fn excel_like_light_gray_rule_is_removed() {
        let mut c = Canvas::new(200, 50, WHITE);
        c.rect(0, 30, 200, 1, gray(212));
        assert!(all(&c.strip().expect("rule"), WHITE));
    }

    #[test]
    fn dark_background_light_rules_removed_background_kept() {
        let bg = [40, 35, 30, 255];
        for t in 1..=3 {
            let mut c = Canvas::new(200, 120, bg);
            c.rect(5, 50, 180, t, gray(200));
            c.rect(100, 0, t, 120, gray(170));
            let out = c.strip().expect("rule found");
            assert!(all(&out, bg), "thickness {t}");
        }
    }

    #[test]
    fn replacement_is_the_average_of_both_sides() {
        // Nền trên 240, dưới 250 (gần nhau), kẻ 100 ở hàng 10.
        let mut c = Canvas::new(100, 20, gray(250));
        c.rect(0, 0, 100, 10, gray(240));
        c.rect(0, 10, 100, 1, gray(100));
        let out = c.strip().expect("rule");
        assert_eq!(Canvas::at(&out.bgra, 100, 50, 10), gray(245));
        assert_eq!(Canvas::at(&out.bgra, 100, 50, 9), gray(240));
        assert_eq!(Canvas::at(&out.bgra, 100, 50, 11), gray(250));
    }

    #[test]
    fn thick_strokes_are_kept() {
        let mut c = Canvas::new(300, 60, WHITE);
        c.rect(10, 20, 250, 5, BLACK);
        c.rect(10, 40, 250, 4, BLACK);
        assert!(c.strip().is_none());
        let mut c = Canvas::new(60, 300, WHITE);
        c.rect(20, 10, 5, 250, BLACK);
        assert!(c.strip().is_none());
    }

    #[test]
    fn short_strokes_and_text_like_shapes_are_kept() {
        let mut c = Canvas::new(400, 80, WHITE);
        // gạch chân ngắn 30 px, dấu gạch ngang, chữ "l" cao 20 px, chân chữ.
        c.rect(10, 30, 30, 1, BLACK);
        c.rect(60, 30, 20, 2, BLACK);
        c.rect(100, 10, 2, 20, BLACK);
        // hàng đỉnh chữ kiểu "zzzzzz": nét 8 px cách nhau 4 px (đầy 2/3 < 4/5).
        for k in 0..20 {
            c.rect(150 + k * 12, 50, 8, 2, BLACK);
        }
        assert!(c.strip().is_none());
    }

    #[test]
    fn no_rules_gives_none() {
        assert!(Canvas::new(300, 200, WHITE).strip().is_none());
        assert!(Canvas::new(300, 200, gray(20)).strip().is_none());
        // Hai nửa sáng/tối: mép giữa không phải đường kẻ.
        let mut c = Canvas::new(300, 200, WHITE);
        c.rect(0, 100, 300, 100, gray(20));
        c.rect(150, 0, 150, 200, gray(90));
        assert!(c.strip().is_none());
    }

    #[test]
    fn tiny_empty_and_invalid_inputs() {
        assert!(strip_rules(&WHITE, 1, 1).is_none());
        assert!(strip_rules(&BLACK, 1, 1).is_none());
        assert!(strip_rules(&[], 0, 0).is_none());
        assert!(strip_rules(&[], 10, 0).is_none());
        assert!(strip_rules(&[0; 12], 2, 2).is_none());
        // 1 hàng, 1 cột: không có hai bên.
        assert!(strip_rules(&BLACK.repeat(100), 100, 1).is_none());
        assert!(strip_rules(&BLACK.repeat(100), 1, 100).is_none());
    }

    #[test]
    fn rules_on_the_image_edges_are_removed() {
        let mut c = Canvas::new(200, 100, WHITE);
        c.rect(0, 0, 200, 1, BLACK);
        c.rect(0, 98, 200, 2, BLACK);
        c.rect(0, 0, 1, 100, BLACK);
        c.rect(199, 0, 1, 100, BLACK);
        assert!(all(&c.strip().expect("edge rules"), WHITE));
    }

    #[test]
    fn grid_with_descenders_crossing_rule() {
        // Bảng 3×3 ô, kẻ 1 px; chân chữ (2×6 px) cắt qua đường ngang ở y = 40.
        let mut c = Canvas::new(400, 130, WHITE);
        for y in [0, 40, 80, 120] {
            c.rect(0, y, 400, 1, gray(70));
        }
        for x in [0, 130, 260, 399] {
            c.rect(x, 0, 1, 121, gray(70));
        }
        let desc: Vec<usize> = (0..12).map(|k| 10 + k * 30).collect();
        for &x in &desc {
            c.rect(x, 36, 2, 6, BLACK);
        }
        let out = c.strip().expect("grid");
        for y in 0..c.h {
            for x in 0..c.w {
                let p = Canvas::at(&out.bgra, c.w, x, y);
                let in_desc = desc.iter().any(|&d| x >= d && x < d + 2) && (36..42).contains(&y);
                if in_desc {
                    assert_eq!(p, BLACK, "descender kept at ({x},{y})");
                } else {
                    assert_eq!(p, WHITE, "rule left at ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn long_underline_is_removed_text_above_kept() {
        let mut c = Canvas::new(300, 40, WHITE);
        // "chữ" giả: các ô 6×12 trên đường gạch chân dài cách chân chữ 2 px.
        for k in 0..20 {
            c.rect(10 + k * 12, 10, 6, 12, BLACK);
        }
        c.rect(10, 24, 240, 1, BLACK);
        let out = c.strip().expect("underline");
        assert_eq!(c.changed(&out), 240);
    }
}
