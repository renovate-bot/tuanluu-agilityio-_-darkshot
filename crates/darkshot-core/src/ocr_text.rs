//! Xử lý chữ và cỡ ảnh cho OCR (không phụ thuộc hệ điều hành).

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
}
