//! Bộ tô che: làm phẳng ô đen đặc (B0 G0 R0 A255) vào một bản sao của ảnh.
//! Không blur, không mosaic, không lớp trong suốt. Ảnh gốc không đổi. Module này không ghi log.

use crate::geometry::Rect;
use crate::image::Image;

/// Điểm ảnh che: đen đặc, alpha 255.
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// Phần nới tối thiểu mỗi phía (px) để phủ viền khử răng cưa của chữ.
pub const MIN_PAD: i32 = 3;

/// Phần nới mỗi phía của một khung: `max(3, ceil(0,15 × chiều cao))`.
pub fn pad_for(r: Rect) -> i32 {
    let h = i64::from(r.h);
    if h <= 0 {
        return MIN_PAD;
    }
    // ceil(15 * h / 100) bằng số nguyên; h ≤ i32::MAX nên không tràn i64.
    let p = (h * 15 + 99) / 100;
    i32::try_from(p).unwrap_or(i32::MAX).max(MIN_PAD)
}

/// Khung nửa mở [x0, x1) × [y0, y1) tính bằng `i64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl Span {
    /// Giao (phần chung có diện tích) hoặc chạm cạnh (chung một đoạn cạnh dài > 0).
    fn joins(&self, o: &Span) -> bool {
        let ox = self.x1.min(o.x1) - self.x0.max(o.x0);
        let oy = self.y1.min(o.y1) - self.y0.max(o.y0);
        ox >= 0 && oy >= 0 && (ox > 0 || oy > 0)
    }

    fn union(&self, o: &Span) -> Span {
        Span {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }
}

/// Nới một khung, kẹp vào w×h; `None` khi khung rỗng/âm hoặc nằm ngoài ảnh.
fn padded_clamped(r: Rect, w: i64, h: i64) -> Option<Span> {
    if r.w <= 0 || r.h <= 0 {
        return None;
    }
    let pad = i64::from(pad_for(r));
    let x0 = (i64::from(r.x) - pad).max(0);
    let y0 = (i64::from(r.y) - pad).max(0);
    let x1 = (i64::from(r.x) + i64::from(r.w) + pad).min(w);
    let y1 = (i64::from(r.y) + i64::from(r.h) + pad).min(h);
    (x1 > x0 && y1 > y0).then_some(Span { x0, y0, x1, y1 })
}

/// Cạnh ảnh dùng cho phép kẹp (kẹp vào `i32::MAX` để kết quả vừa `Rect`).
fn limit(v: u32) -> i64 {
    i64::from(v).min(i64::from(i32::MAX))
}

/// Số khung của `rects` còn phủ ít nhất một điểm ảnh của ảnh w×h sau khi nới và kẹp.
pub(crate) fn covered_count(rects: &[Rect], w: u32, h: u32) -> usize {
    let (w, h) = (limit(w), limit(h));
    rects
        .iter()
        .filter(|r| padded_clamped(**r, w, h).is_some())
        .count()
}

/// Nới, kẹp vào w×h, bỏ khung rỗng/âm, gộp khung giao/chạm. Kết quả rời nhau, trong ảnh.
pub fn normalize(rects: &[Rect], w: u32, h: u32) -> Vec<Rect> {
    let (w, h) = (limit(w), limit(h));
    let mut spans: Vec<Span> = rects
        .iter()
        .filter_map(|r| padded_clamped(*r, w, h))
        .collect();
    // Gộp lặp tới điểm cố định: mỗi lần gộp giảm số khung nên vòng lặp dừng.
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < spans.len() {
            let mut j = i + 1;
            while j < spans.len() {
                if spans[i].joins(&spans[j]) {
                    spans[i] = spans[i].union(&spans[j]);
                    spans.swap_remove(j);
                    changed = true;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
    spans.sort_unstable_by_key(|s| (s.y0, s.x0));
    spans
        .into_iter()
        .map(|s| Rect {
            x: s.x0 as i32,
            y: s.y0 as i32,
            w: (s.x1 - s.x0) as i32,
            h: (s.y1 - s.y0) as i32,
        })
        .collect()
}

/// Lỗi tô che. Không mang chuỗi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedactError {
    InvalidImage,
    OutOfMemory,
}

/// Ảnh mới: mọi điểm trong `normalize(rects)` = B0 G0 R0 A255; ngoài giữ nguyên. `src` không đổi.
pub fn flatten(src: &Image, rects: &[Rect]) -> Result<Image, RedactError> {
    if !src.is_valid() {
        return Err(RedactError::InvalidImage);
    }
    let mut bgra: Vec<u8> = Vec::new();
    bgra.try_reserve_exact(src.bgra.len())
        .map_err(|_| RedactError::OutOfMemory)?;
    let stride = src.width as usize * 4;
    for row in src.bgra.chunks_exact(stride) {
        bgra.extend_from_slice(row);
    }
    for r in normalize(rects, src.width, src.height) {
        let (x0, x1) = (r.x as usize * 4, (r.x + r.w) as usize * 4);
        for y in r.y as usize..(r.y + r.h) as usize {
            let start = y * stride;
            for p in bgra[start + x0..start + x1].as_chunks_mut::<4>().0 {
                *p = BLACK;
            }
        }
    }
    Ok(Image {
        width: src.width,
        height: src.height,
        bgra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{encode_jpeg, encode_png};
    use sha2::{Digest, Sha256};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    /// Ảnh tổng hợp nhiều màu, alpha thay đổi (có điểm alpha < 255).
    fn sample(w: u32, h: u32) -> Image {
        let mut bgra = Vec::with_capacity(w as usize * h as usize * 4);
        for y in 0..h {
            for x in 0..w {
                bgra.extend_from_slice(&[
                    (x * 37 + y * 11 + 1) as u8,
                    (x * 5 + y * 71 + 2) as u8,
                    (x * y * 13 + 3) as u8,
                    (x + y) as u8 | 0x10,
                ]);
            }
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    fn px(img: &Image, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * img.width as usize + x as usize) * 4;
        img.bgra[i..i + 4].try_into().unwrap()
    }

    fn inside(rs: &[Rect], x: u32, y: u32) -> bool {
        let (x, y) = (i64::from(x), i64::from(y));
        rs.iter().any(|r| {
            x >= i64::from(r.x)
                && x < i64::from(r.x) + i64::from(r.w)
                && y >= i64::from(r.y)
                && y < i64::from(r.y) + i64::from(r.h)
        })
    }

    /// Bộ sinh số giả ngẫu nhiên có hạt giống cố định (xorshift64*).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn range(&mut self, lo: i32, hi: i32) -> i32 {
            lo + (self.next() % (hi - lo) as u64) as i32
        }
    }

    fn assert_flatten_matches(src: &Image, rects: &[Rect], out: &Image) {
        let norm = normalize(rects, src.width, src.height);
        assert_eq!((out.width, out.height), (src.width, src.height));
        for y in 0..src.height {
            for x in 0..src.width {
                if inside(&norm, x, y) {
                    assert_eq!(px(out, x, y), BLACK, "trong khung tại ({x},{y})");
                } else {
                    assert_eq!(px(out, x, y), px(src, x, y), "ngoài khung tại ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn pad_rule() {
        assert_eq!(MIN_PAD, 3);
        assert_eq!(pad_for(rect(0, 0, 5, 1)), 3);
        assert_eq!(pad_for(rect(0, 0, 5, 20)), 3);
        assert_eq!(pad_for(rect(0, 0, 5, 21)), 4);
        assert_eq!(pad_for(rect(0, 0, 5, 100)), 15);
        assert_eq!(pad_for(rect(0, 0, 5, 101)), 16);
        assert_eq!(pad_for(rect(0, 0, 5, 0)), 3);
        assert_eq!(pad_for(rect(0, 0, 5, -40)), 3);
        assert_eq!(pad_for(rect(0, 0, 5, i32::MAX)), 322_122_548);
    }

    #[test]
    fn normalize_pads_each_side() {
        assert_eq!(
            normalize(&[rect(10, 10, 5, 20)], 100, 100),
            vec![rect(7, 7, 11, 26)]
        );
        assert_eq!(
            normalize(&[rect(20, 20, 10, 40)], 100, 100),
            vec![rect(14, 14, 22, 52)]
        );
    }

    #[test]
    fn normalize_clamps_to_image() {
        assert_eq!(
            normalize(&[rect(-5, -5, 10, 10)], 100, 100),
            vec![rect(0, 0, 8, 8)]
        );
        assert_eq!(
            normalize(&[rect(95, 90, 10, 20)], 100, 100),
            vec![rect(92, 87, 8, 13)]
        );
    }

    #[test]
    fn normalize_drops_empty_negative_and_outside() {
        let rs = [
            rect(10, 10, 0, 5),
            rect(10, 10, 5, 0),
            rect(10, 10, -5, 5),
            rect(10, 10, 5, -5),
            rect(200, 200, 5, 5),
            rect(103, 10, 5, 5),
            rect(-50, -50, 5, 5),
            rect(-8, 10, 5, 5),
        ];
        assert!(normalize(&rs, 100, 100).is_empty());
        assert!(normalize(&[], 100, 100).is_empty());
        assert!(normalize(&[rect(0, 0, 5, 5)], 0, 0).is_empty());
    }

    #[test]
    fn normalize_extreme_values_do_not_overflow() {
        assert_eq!(
            normalize(&[rect(0, 0, i32::MAX, i32::MAX)], 100, 50),
            vec![rect(0, 0, 100, 50)]
        );
        assert!(normalize(&[rect(i32::MAX, i32::MAX, i32::MAX, i32::MAX)], 100, 50).is_empty());
        assert_eq!(
            normalize(&[rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX)], 100, 50),
            vec![rect(0, 0, 100, 50)]
        );
        assert!(normalize(&[rect(i32::MIN, i32::MIN, 10, 10)], 100, 50).is_empty());
        assert!(normalize(&[rect(i32::MIN, 0, i32::MIN, 10)], 100, 50).is_empty());
        // Ảnh cạnh vượt i32::MAX: kết quả vẫn vừa Rect.
        let r = normalize(&[rect(0, 0, i32::MAX, 10)], u32::MAX, 100);
        assert_eq!(r, vec![rect(0, 0, i32::MAX, 13)]);
    }

    #[test]
    fn normalize_merges_overlap_and_touch() {
        // [7,17)² và [17,27)×[7,17) chạm cạnh x = 17.
        assert_eq!(
            normalize(&[rect(10, 10, 4, 4), rect(20, 10, 4, 4)], 100, 100),
            vec![rect(7, 7, 20, 10)]
        );
        // Giao nhau.
        assert_eq!(
            normalize(&[rect(10, 10, 4, 4), rect(12, 12, 4, 4)], 100, 100),
            vec![rect(7, 7, 12, 12)]
        );
        // Rời nhau giữ riêng, sắp theo (y, x).
        assert_eq!(
            normalize(&[rect(50, 50, 4, 4), rect(10, 10, 4, 4)], 100, 100),
            vec![rect(7, 7, 10, 10), rect(47, 47, 10, 10)]
        );
        // Chỉ chạm góc: không chung cạnh, giữ riêng.
        assert_eq!(
            normalize(&[rect(10, 10, 4, 4), rect(20, 20, 4, 4)], 100, 100),
            vec![rect(7, 7, 10, 10), rect(17, 17, 10, 10)]
        );
    }

    #[test]
    fn normalize_merges_to_fixed_point() {
        // B chỉ giao khung bao của A và C, không giao riêng A hay C.
        let b = rect(22, 3, 4, 4); // [19,29)×[0,10)
        let a = rect(10, 10, 4, 4); // [7,17)²
        let c = rect(16, 16, 4, 4); // [13,23)²
        assert_eq!(normalize(&[b, a, c], 100, 100), vec![rect(7, 0, 22, 23)]);
        assert_eq!(normalize(&[a, c, b], 100, 100), vec![rect(7, 0, 22, 23)]);
    }

    #[test]
    fn normalize_random_is_disjoint_inside_and_covers_input() {
        let mut rng = Rng(0x5EED_0129);
        let (w, h) = (300u32, 200u32);
        let rs: Vec<Rect> = (0..1000)
            .map(|_| {
                rect(
                    rng.range(-40, 340),
                    rng.range(-40, 240),
                    rng.range(-3, 25),
                    rng.range(-3, 25),
                )
            })
            .collect();
        let norm = normalize(&rs, w, h);
        for (i, a) in norm.iter().enumerate() {
            assert!(a.w > 0 && a.h > 0);
            assert!(a.x >= 0 && a.y >= 0);
            assert!(a.right() <= w as i32 && a.bottom() <= h as i32);
            for b in &norm[i + 1..] {
                let ox = a.right().min(b.right()) - a.x.max(b.x);
                let oy = a.bottom().min(b.bottom()) - a.y.max(b.y);
                assert!(!(ox > 0 && oy > 0), "giao nhau: {a:?} {b:?}");
                assert!(
                    !(ox >= 0 && oy >= 0 && (ox > 0 || oy > 0)),
                    "chạm: {a:?} {b:?}"
                );
            }
        }
        for r in &rs {
            if let Some(s) = padded_clamped(*r, i64::from(w), i64::from(h)) {
                assert!(
                    norm.iter().any(|n| i64::from(n.x) <= s.x0
                        && i64::from(n.y) <= s.y0
                        && i64::from(n.right()) >= s.x1
                        && i64::from(n.bottom()) >= s.y1),
                    "khung {r:?} không được phủ"
                );
            }
        }
    }

    #[test]
    fn flatten_blacks_inside_keeps_outside() {
        let src = sample(64, 48);
        let rs = [rect(5, 5, 10, 8), rect(40, 30, 20, 30), rect(-3, 40, 6, 2)];
        let out = flatten(&src, &rs).unwrap();
        assert_flatten_matches(&src, &rs, &out);
        assert_eq!(px(&out, 2, 2), BLACK);
        assert_eq!(px(&out, 17, 15), BLACK);
        assert_ne!(px(&out, 18, 16), BLACK);
    }

    #[test]
    fn flatten_does_not_change_source() {
        let src = sample(80, 60);
        let copy = src.clone();
        let before = Sha256::digest(&src.bgra);
        let out = flatten(&src, &[rect(10, 10, 30, 20)]).unwrap();
        assert_eq!(Sha256::digest(&src.bgra), before);
        assert_eq!(src, copy);
        assert_ne!(out, src);
    }

    #[test]
    fn flatten_translucent_source_becomes_opaque() {
        let mut src = sample(20, 20);
        for p in src.bgra.as_chunks_mut::<4>().0 {
            p[3] = 0x40;
        }
        let out = flatten(&src, &[rect(8, 8, 4, 4)]).unwrap();
        assert_eq!(px(&out, 10, 10), BLACK);
        assert_eq!(px(&out, 0, 0)[3], 0x40);
        assert_flatten_matches(&src, &[rect(8, 8, 4, 4)], &out);
    }

    #[test]
    fn flatten_one_by_one_image() {
        let src = Image {
            width: 1,
            height: 1,
            bgra: vec![9, 8, 7, 6],
        };
        assert_eq!(flatten(&src, &[rect(0, 0, 1, 1)]).unwrap().bgra, BLACK);
        assert_eq!(flatten(&src, &[rect(2, 0, 1, 1)]).unwrap().bgra, BLACK);
        assert_eq!(flatten(&src, &[rect(4, 0, 1, 1)]).unwrap(), src);
        assert_eq!(flatten(&src, &[]).unwrap(), src);
    }

    #[test]
    fn flatten_tiny_rect_covers_at_least_7x7() {
        let src = sample(100, 100);
        let out = flatten(&src, &[rect(50, 50, 1, 1)]).unwrap();
        for y in 47..54 {
            for x in 47..54 {
                assert_eq!(px(&out, x, y), BLACK, "({x},{y})");
            }
        }
        for i in 46..55 {
            assert_eq!(px(&out, 46, i), px(&src, 46, i));
            assert_eq!(px(&out, 54, i), px(&src, 54, i));
            assert_eq!(px(&out, i, 46), px(&src, i, 46));
            assert_eq!(px(&out, i, 54), px(&src, i, 54));
        }
    }

    #[test]
    fn flatten_extreme_rects() {
        let src = sample(30, 20);
        let all = flatten(&src, &[rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX)]).unwrap();
        assert!(all.bgra.as_chunks::<4>().0.iter().all(|p| *p == BLACK));
        let none = flatten(
            &src,
            &[
                rect(i32::MAX, i32::MAX, i32::MAX, i32::MAX),
                rect(0, 0, -1, -1),
                rect(5, 5, 0, 9),
            ],
        )
        .unwrap();
        assert_eq!(none, src);
    }

    #[test]
    fn flatten_rejects_invalid_image() {
        let bad = Image {
            width: 4,
            height: 4,
            bgra: vec![0; 10],
        };
        assert_eq!(flatten(&bad, &[]), Err(RedactError::InvalidImage));
        let empty = Image {
            width: 0,
            height: 0,
            bgra: Vec::new(),
        };
        assert_eq!(
            flatten(&empty, &[rect(0, 0, 1, 1)]),
            Err(RedactError::InvalidImage)
        );
    }

    #[test]
    fn flatten_thousand_random_rects_matches_reference() {
        let mut rng = Rng(0x0129_0129_0129);
        let src = sample(320, 240);
        let rs: Vec<Rect> = (0..1000)
            .map(|_| {
                rect(
                    rng.range(-30, 350),
                    rng.range(-30, 270),
                    rng.range(-2, 6),
                    rng.range(-2, 6),
                )
            })
            .collect();
        let out = flatten(&src, &rs).unwrap();
        assert_flatten_matches(&src, &rs, &out);
        // Tham chiếu độc lập: mọi điểm của từng khung đã nới (chưa gộp) đều đen.
        for r in &rs {
            if r.w <= 0 || r.h <= 0 {
                continue;
            }
            let p = pad_for(*r);
            for y in (r.y - p).max(0)..(r.y + r.h + p).min(240) {
                for x in (r.x - p).max(0)..(r.x + r.w + p).min(320) {
                    assert_eq!(px(&out, x as u32, y as u32), BLACK);
                }
            }
        }
    }

    /// Các loại chunk của một tệp PNG (duyệt thô).
    fn png_chunks(data: &[u8]) -> Vec<[u8; 4]> {
        assert_eq!(&data[..8], b"\x89PNG\r\n\x1a\n");
        let mut out = Vec::new();
        let mut i = 8;
        while i + 8 <= data.len() {
            let len = u32::from_be_bytes(data[i..i + 4].try_into().unwrap()) as usize;
            out.push(data[i + 4..i + 8].try_into().unwrap());
            i += 12 + len;
        }
        assert_eq!(i, data.len());
        out
    }

    #[test]
    fn png_of_flattened_is_pure_black_and_has_no_text_chunks() {
        let src = sample(40, 30);
        let rs = [rect(5, 5, 6, 6), rect(25, 15, 10, 10)];
        let out = flatten(&src, &rs).unwrap();
        let bytes = encode_png(&out).unwrap();
        let chunks = png_chunks(&bytes);
        for bad in [b"tEXt", b"iTXt", b"zTXt", b"eXIf"] {
            assert!(!chunks.contains(bad), "{:?}", bad);
        }
        assert_eq!(chunks.first(), Some(b"IHDR"));
        assert_eq!(chunks.last(), Some(b"IEND"));

        let dec = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = dec.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgb);
        assert_eq!((info.width, info.height), (40, 30));
        let norm = normalize(&rs, 40, 30);
        let mut blacks = 0;
        for y in 0..30u32 {
            for x in 0..40u32 {
                let i = (y as usize * 40 + x as usize) * 3;
                let rgb = &buf[i..i + 3];
                if inside(&norm, x, y) {
                    assert_eq!(rgb, [0, 0, 0], "({x},{y})");
                    blacks += 1;
                } else {
                    let s = px(&src, x, y);
                    assert_eq!(rgb, [s[2], s[1], s[0]]);
                }
            }
        }
        assert!(blacks >= 2 * 7 * 7);
    }

    /// Các marker JPEG từ SOI đến SOS.
    fn jpeg_markers(data: &[u8]) -> Vec<u8> {
        assert_eq!(&data[..2], &[0xFF, 0xD8]);
        let mut out = Vec::new();
        let mut i = 2;
        while i + 4 <= data.len() {
            assert_eq!(data[i], 0xFF);
            let m = data[i + 1];
            let len = usize::from(u16::from_be_bytes([data[i + 2], data[i + 3]]));
            out.push(m);
            i += 2 + len;
            if m == 0xDA {
                break;
            }
        }
        out
    }

    #[test]
    fn jpeg_of_flattened_has_no_app1_or_comment() {
        let src = sample(48, 32);
        let out = flatten(&src, &[rect(8, 8, 12, 12)]).unwrap();
        for q in [90, 50] {
            let bytes = encode_jpeg(&out, q).unwrap();
            let markers = jpeg_markers(&bytes);
            assert!(markers.contains(&0xDA), "q={q}");
            assert!(!markers.contains(&0xE1), "APP1 q={q}");
            assert!(!markers.contains(&0xFE), "COM q={q}");
            assert_eq!(&bytes[bytes.len() - 2..], &[0xFF, 0xD9]);
        }
    }

    /// Ảnh lớn tổng hợp 8000×20000 (~640 MB mỗi bản). Chạy tay:
    /// `cargo test -p darkshot-core --release -- --ignored flatten_large_image`.
    #[test]
    #[ignore]
    fn flatten_large_image() {
        let (w, h) = (8000u32, 20000u32);
        let mut bgra = vec![0u8; w as usize * h as usize * 4];
        for (i, p) in bgra.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            *p = [(i % 251) as u8, (i % 241) as u8, 200, 255];
        }
        let src = Image {
            width: w,
            height: h,
            bgra,
        };
        let rs: Vec<Rect> = (0..400)
            .map(|k| rect((k * 37) % 7900, k * 50, 60, 20))
            .collect();
        let out = flatten(&src, &rs).unwrap();
        let norm = normalize(&rs, w, h);
        for r in &norm {
            for y in [r.y, r.bottom() - 1] {
                for x in [r.x, r.right() - 1] {
                    assert_eq!(px(&out, x as u32, y as u32), BLACK);
                }
            }
        }
        assert_eq!(px(&out, 7999, 19999), px(&src, 7999, 19999));
        assert_eq!(out.bgra.len(), src.bgra.len());
    }
}
