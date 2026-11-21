//! Xử lý chữ và cỡ ảnh cho OCR (không phụ thuộc hệ điều hành).

use crate::geometry::Rect;

/// Cạnh ngắn nhỏ hơn mức này thì phóng 2× trước khi OCR.
pub const SMALL_EDGE: u32 = 40;

/// Trim từng dòng, bỏ dòng rỗng ở đầu/cuối (giữ dòng rỗng giữa), nối "\r\n".
pub fn join_lines(lines: &[String]) -> String {
    let trimmed: Vec<&str> = lines.iter().map(|l| l.trim()).collect();
    let Some(first) = trimmed.iter().position(|l| !l.is_empty()) else {
        return String::new();
    };
    let last = trimmed.iter().rposition(|l| !l.is_empty()).unwrap_or(first);
    trimmed[first..=last].join("\r\n")
}

/// Cỡ mới trước khi OCR: cạnh dài > max_dim → thu giữ tỉ lệ (làm tròn, mỗi cạnh ≥ 1);
/// cạnh ngắn < SMALL_EDGE và 2× vẫn ≤ max_dim → (2w, 2h); còn lại None.
pub fn ocr_scale(w: u32, h: u32, max_dim: u32) -> Option<(u32, u32)> {
    let (long, short) = (w.max(h), w.min(h));
    if long > max_dim {
        let f = |v: u32| -> u32 {
            let r = (u64::from(v) * u64::from(max_dim) + u64::from(long) / 2) / u64::from(long);
            (r as u32).max(1)
        };
        return Some((f(w), f(h)));
    }
    if short < SMALL_EDGE && u64::from(long) * 2 <= u64::from(max_dim) {
        return Some((w * 2, h * 2));
    }
    None
}

/// Số dòng không rỗng.
pub fn line_count(text: &str) -> usize {
    text.lines().filter(|l| !l.trim().is_empty()).count()
}

/// Một dòng chữ nhận dạng được và khung của nó (px ảnh gốc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrLine {
    pub text: String,
    pub rect: Rect,
}

/// Cách ghép chữ theo các công tắc trong cửa sổ OCR.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextOptions {
    pub single_line: bool,
    pub paragraphs: bool,
}

/// Chỉ số các dòng mở đoạn; luôn có 0 nếu `lines` không rỗng.
pub fn paragraphs(_lines: &[OcrLine]) -> Vec<usize> {
    unimplemented!()
}

/// Ghép các dòng theo `idx` (sắp xếp, bỏ trùng và chỉ số ngoài dải).
pub fn compose(_lines: &[OcrLine], _idx: &[usize], _o: TextOptions) -> String {
    unimplemented!()
}

/// `compose` với mọi chỉ số.
pub fn compose_all(_lines: &[OcrLine], _o: TextOptions) -> String {
    unimplemented!()
}

/// Dòng đầu tiên (chỉ số nhỏ nhất) có khung, nới 2 px mỗi phía, chứa điểm.
pub fn line_at(_lines: &[OcrLine], _x: i32, _y: i32) -> Option<usize> {
    unimplemented!()
}

/// Đổi khung trong ảnh cỡ (fw,fh) về ảnh cỡ (tw,th): tỉ lệ riêng từng trục,
/// làm tròn ra ngoài, kẹp trong (0,0,tw,th).
pub fn map_rect(_r: Rect, _fw: u32, _fh: u32, _tw: u32, _th: u32) -> Rect {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn join_lines_trims_and_drops_edge_blanks() {
        assert_eq!(join_lines(&v(&["  a ", "b\t"])), "a\r\nb");
        assert_eq!(
            join_lines(&v(&["", "  ", "a", "", "b", " ", ""])),
            "a\r\n\r\nb"
        );
        assert_eq!(join_lines(&v(&["", "  "])), "");
        assert_eq!(join_lines(&[]), "");
    }

    #[test]
    fn ocr_scale_cases() {
        assert_eq!(ocr_scale(800, 600, 10000), None);
        assert_eq!(ocr_scale(20000, 100, 10000), Some((10000, 50)));
        assert_eq!(ocr_scale(300, 30, 10000), Some((600, 60)));
        assert_eq!(ocr_scale(300, 30, 500), None);
        assert_eq!(ocr_scale(40, 40, 10000), None);
        let (w, h) = ocr_scale(100000, 1, 10000).unwrap();
        assert_eq!((w, h), (10000, 1));
    }

    #[test]
    fn line_count_counts_non_empty() {
        assert_eq!(line_count(""), 0);
        assert_eq!(line_count("a\r\n\r\n b \r\n  "), 2);
    }

    fn ln(text: &str, y: i32, h: i32) -> OcrLine {
        OcrLine {
            text: text.to_string(),
            rect: Rect { x: 0, y, w: 100, h },
        }
    }

    fn rc(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn abc() -> Vec<OcrLine> {
        vec![ln("a", 0, 10), ln("b", 12, 10), ln("c", 24, 10)]
    }

    fn two_paras() -> Vec<OcrLine> {
        vec![ln("a", 0, 10), ln("b", 12, 10), ln("c", 40, 10)]
    }

    #[test]
    fn compose_basic_options() {
        let l = abc();
        let none = TextOptions::default();
        assert_eq!(compose_all(&l, none), "a\r\nb\r\nc");
        let one = TextOptions {
            single_line: true,
            paragraphs: false,
        };
        assert_eq!(compose_all(&l, one), "a b c");
        assert_eq!(compose(&l, &[2, 0, 0, 9], none), "a\r\nc");
        assert_eq!(compose(&l, &[], none), "");
    }

    #[test]
    fn compose_trims_lines() {
        let l = vec![ln("  a ", 0, 10), ln("\tb", 12, 10)];
        assert_eq!(compose_all(&l, TextOptions::default()), "a\r\nb");
    }

    #[test]
    fn compose_paragraphs() {
        let l = two_paras();
        let p = TextOptions {
            single_line: false,
            paragraphs: true,
        };
        assert_eq!(compose_all(&l, p), "a\r\nb\r\n\r\nc");
        assert_eq!(compose(&l, &[2], p), "c");
        let both = TextOptions {
            single_line: true,
            paragraphs: true,
        };
        assert_eq!(compose_all(&l, both), "a b c");
    }

    #[test]
    fn paragraphs_cases() {
        assert_eq!(paragraphs(&[]), Vec::<usize>::new());
        assert_eq!(paragraphs(&[ln("a", 5, 10)]), vec![0]);
        assert_eq!(paragraphs(&abc()), vec![0]);
        assert_eq!(paragraphs(&two_paras()), vec![0, 2]);
        // Khe đúng 0,8·m (8 px khi m=10) chưa mở đoạn; 9 px thì mở.
        let edge = vec![ln("a", 0, 10), ln("b", 18, 10)];
        assert_eq!(paragraphs(&edge), vec![0]);
        let over = vec![ln("a", 0, 10), ln("b", 19, 10)];
        assert_eq!(paragraphs(&over), vec![0, 1]);
        // Cột hai: top nhảy lên.
        let col = vec![ln("a", 0, 10), ln("b", 12, 10), ln("c", 0, 10)];
        assert_eq!(paragraphs(&col), vec![0, 2]);
        // Lệch top 0,3·m trên cùng hàng: không mở.
        let same = vec![ln("a", 3, 10), ln("b", 0, 10)];
        assert_eq!(paragraphs(&same), vec![0]);
    }

    #[test]
    fn line_at_cases() {
        let l = vec![
            OcrLine {
                text: "a".into(),
                rect: rc(10, 10, 20, 10),
            },
            OcrLine {
                text: "b".into(),
                rect: rc(15, 12, 20, 10),
            },
        ];
        assert_eq!(line_at(&l, 12, 12), Some(0));
        assert_eq!(line_at(&l, 8, 10), Some(0)); // nới 2 px
        assert_eq!(line_at(&l, 7, 10), None); // 3 px
        assert_eq!(line_at(&l, 20, 15), Some(0)); // chồng lấn -> nhỏ
        assert_eq!(line_at(&l, 33, 21), Some(1));
        assert_eq!(line_at(&[], 0, 0), None);
    }

    #[test]
    fn map_rect_cases() {
        let r = rc(400, 100, 200, 50);
        assert_eq!(map_rect(r, 4000, 1000, 2000, 500), rc(200, 50, 100, 25));
        assert_eq!(map_rect(rc(200, 50, 100, 25), 2000, 500, 4000, 1000), r);
        // Làm tròn ra ngoài: 1x1 lẻ vẫn >= 1 px.
        assert_eq!(map_rect(rc(3, 3, 1, 1), 10, 10, 5, 5), rc(1, 1, 1, 1));
        // Kẹp mép.
        assert_eq!(
            map_rect(rc(90, 90, 30, 30), 100, 100, 50, 50),
            rc(45, 45, 5, 5)
        );
        assert_eq!(
            map_rect(rc(-10, -10, 20, 20), 100, 100, 100, 100),
            rc(0, 0, 10, 10)
        );
    }
}
