//! Dựng chữ một dòng OCR cho bộ dò: hai bản (có khoảng trắng / ghép từ bị tách),
//! bảng `(start, end, word)` theo chỉ số `char`, và khung của một khoảng ký tự.
//! Bộ đệm chữ ở đây bị ghi đè bằng 0 khi thả.

use super::model::Mapping;
use crate::geometry::Rect;
use crate::ocr_text::OcrLine;

/// Ghi đè chữ bằng byte 0 rồi làm rỗng (tốt nhất có thể, không hứa trước bộ cấp phát).
pub fn wipe(s: &mut String) {
    // SAFETY: byte 0 là UTF-8 hợp lệ (U+0000), nên chuỗi vẫn hợp lệ sau khi ghi đè.
    unsafe { s.as_bytes_mut() }.fill(0);
    s.clear();
}

/// Ghi đè một bộ đệm ký tự bằng `'\0'` rồi làm rỗng.
pub(crate) fn wipe_chars(v: &mut Vec<char>) {
    v.fill('\0');
    v.clear();
}

/// Ghi đè một bộ đệm byte bằng 0 rồi làm rỗng.
pub(crate) fn wipe_bytes(v: &mut Vec<u8>) {
    v.fill(0);
    v.clear();
}

/// Ký tự thuộc bộ token `[A-Za-z0-9_\-.+/=]`.
pub(crate) fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+' | '/' | '=')
}

/// Chuẩn hóa Unicode tối thiểu, một ký tự đổi một ký tự (giữ chỉ số).
pub(crate) fn normalize(c: char) -> char {
    match c {
        '\u{2010}'..='\u{2015}' | '\u{2212}' | '\u{FE58}' | '\u{FE63}' | '\u{FF0D}' => '-',
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' => '\'',
        '\u{201C}'..='\u{201F}' | '\u{2033}' => '"',
        '\u{FF1A}' => ':',
        '\u{FF1D}' => '=',
        '\u{200B}' | '\u{FEFF}' => ' ',
        c if c.is_whitespace() => ' ',
        c => c,
    }
}

/// Chữ thường, bỏ dấu tiếng Việt (một ký tự đổi một ký tự). Dấu tổ hợp giữ nguyên;
/// bộ so nhãn bỏ qua chúng.
pub(crate) fn fold(c: char) -> char {
    let c = if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        let mut it = c.to_lowercase();
        match (it.next(), it.next()) {
            (Some(l), None) => l,
            _ => c,
        }
    };
    if c.is_ascii() {
        return c;
    }
    let u = c as u32;
    // Latin Extended Additional, khối tiếng Việt U+1EA0..U+1EF9 (từng cặp hoa/thường).
    if (0x1EA0..=0x1EF9).contains(&u) {
        return match u {
            0x1EA0..=0x1EB7 => 'a',
            0x1EB8..=0x1EC7 => 'e',
            0x1EC8..=0x1ECB => 'i',
            0x1ECC..=0x1EE3 => 'o',
            0x1EE4..=0x1EF1 => 'u',
            _ => 'y',
        };
    }
    match u {
        0xE0..=0xE5 | 0x103 => 'a',
        0xE8..=0xEB => 'e',
        0xEC..=0xEF | 0x129 => 'i',
        0xF2..=0xF6 | 0x1A1 => 'o',
        0xF9..=0xFC | 0x169 | 0x1B0 => 'u',
        0xFD | 0xFF => 'y',
        0x111 => 'd',
        _ => c,
    }
}

/// Dấu tổ hợp (bỏ qua khi so nhãn).
pub(crate) fn is_combining(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

/// Một bản dựng chữ của dòng.
#[derive(Default)]
pub struct Version {
    /// Chữ đã chuẩn hóa.
    pub chars: Vec<char>,
    /// `chars` viết thường, bỏ dấu (cùng độ dài).
    pub folded: Vec<char>,
    /// `(char_start, char_end, word_index)`; rỗng khi dòng không có từ.
    pub map: Vec<(usize, usize, usize)>,
}

impl Drop for Version {
    fn drop(&mut self) {
        wipe_chars(&mut self.chars);
        wipe_chars(&mut self.folded);
    }
}

/// Chữ một dòng cho bộ dò.
pub struct LineText {
    pub spaced: Version,
    pub joined: Version,
    rect: Rect,
    word_rects: Vec<Rect>,
}

/// Khung từ dùng được: cỡ dương, góc không âm.
fn valid(r: Rect) -> bool {
    r.w > 0 && r.h > 0 && r.x >= 0 && r.y >= 0
}

/// Hợp hai khung.
pub(crate) fn union(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let r = a.right().max(b.right());
    let btm = a.bottom().max(b.bottom());
    Rect {
        x,
        y,
        w: r - x,
        h: btm - y,
    }
}

impl LineText {
    pub fn build(line: &OcrLine) -> LineText {
        let mut spaced = Version::default();
        let mut joined = Version::default();
        let word_rects: Vec<Rect> = line.words.iter().map(|w| w.rect).collect();
        if line.words.is_empty() {
            spaced.chars = line.text.chars().map(normalize).collect();
            joined.chars = spaced.chars.clone();
        } else {
            let h = i64::from(line.rect.h.max(0));
            let mut prev: Option<(char, Rect)> = None;
            for (wi, w) in line.words.iter().enumerate() {
                let mut chars: Vec<char> = w.text.chars().map(normalize).collect();
                let first = chars.first().copied();
                if let Some(first) = first {
                    if !spaced.chars.is_empty() {
                        spaced.chars.push(' ');
                    }
                    let glue = match prev {
                        Some((last, pr)) => {
                            let gap = i64::from(w.rect.x) - i64::from(pr.right());
                            valid(pr)
                                && valid(w.rect)
                                && gap * 100 < 35 * h
                                && is_token_char(last)
                                && is_token_char(first)
                        }
                        None => true,
                    };
                    if !glue {
                        joined.chars.push(' ');
                    }
                    let s0 = spaced.chars.len();
                    spaced.chars.extend_from_slice(&chars);
                    spaced.map.push((s0, spaced.chars.len(), wi));
                    let j0 = joined.chars.len();
                    joined.chars.extend_from_slice(&chars);
                    joined.map.push((j0, joined.chars.len(), wi));
                    prev = chars.last().map(|&c| (c, w.rect));
                }
                wipe_chars(&mut chars);
            }
        }
        spaced.folded = spaced.chars.iter().map(|&c| fold(c)).collect();
        joined.folded = joined.chars.iter().map(|&c| fold(c)).collect();
        LineText {
            spaced,
            joined,
            rect: line.rect,
            word_rects,
        }
    }

    pub fn version(&self, joined: bool) -> &Version {
        if joined {
            &self.joined
        } else {
            &self.spaced
        }
    }

    /// Hai bản dựng khác nhau (có ghép từ).
    pub fn has_join(&self) -> bool {
        self.spaced.chars.len() != self.joined.chars.len()
    }

    /// Khung của khoảng ký tự `[start, end)`: hợp khung các từ giao khoảng đó.
    /// Không có từ, không ánh xạ được, hoặc có từ khung hỏng → khung cả dòng, `Mapping::Line`.
    pub fn span_rect(&self, joined: bool, start: usize, end: usize) -> (Rect, Mapping) {
        let v = self.version(joined);
        let mut acc: Option<Rect> = None;
        for &(ws, we, wi) in &v.map {
            if ws < end && we > start {
                let r = self.word_rects[wi];
                if !valid(r) {
                    return (self.rect, Mapping::Line);
                }
                acc = Some(match acc {
                    Some(a) => union(a, r),
                    None => r,
                });
            }
        }
        match acc {
            Some(r) => (r, Mapping::Word),
            None => (self.rect, Mapping::Line),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr_text::OcrWord;

    fn rc(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn line(words: &[(&str, Rect)]) -> OcrLine {
        let mut r = words[0].1;
        for w in words {
            r = union(r, w.1);
        }
        OcrLine {
            text: words.iter().map(|w| w.0).collect::<Vec<_>>().join(" "),
            rect: r,
            words: words
                .iter()
                .map(|w| OcrWord {
                    text: w.0.into(),
                    rect: w.1,
                })
                .collect(),
        }
    }

    fn s(v: &[char]) -> String {
        v.iter().collect()
    }

    #[test]
    fn char_map_uses_char_indices_not_bytes() {
        let l = line(&[("mật", rc(0, 0, 30, 20)), ("khẩu:", rc(40, 0, 50, 20))]);
        let t = LineText::build(&l);
        assert_eq!(t.spaced.map, vec![(0, 3, 0), (4, 9, 1)]);
        assert_eq!(s(&t.spaced.folded), "mat khau:");
        assert_eq!(t.span_rect(false, 4, 9), (rc(40, 0, 50, 20), Mapping::Word));
        assert_eq!(t.span_rect(false, 2, 5), (rc(0, 0, 90, 20), Mapping::Word));
    }

    #[test]
    fn joins_split_token_words_only_when_gap_small() {
        // cao dòng 20 → ngưỡng 7 px
        let l = line(&[
            ("key:", rc(0, 0, 40, 20)),
            ("AKIA", rc(50, 0, 40, 20)),
            ("ABCD", rc(92, 0, 40, 20)),
            ("EFGH", rc(140, 0, 40, 20)),
        ]);
        let t = LineText::build(&l);
        assert_eq!(s(&t.spaced.chars), "key: AKIA ABCD EFGH");
        // ':' không thuộc bộ token → không ghép; khe 2 px → ghép; khe 8 px → không.
        assert_eq!(s(&t.joined.chars), "key: AKIAABCD EFGH");
        assert!(t.has_join());
        assert_eq!(t.joined.map[2], (9, 13, 2));
        assert_eq!(t.span_rect(true, 5, 13), (rc(50, 0, 82, 20), Mapping::Word));
    }

    #[test]
    fn falls_back_to_line_rect() {
        let l = line(&[("a", rc(0, 0, 10, 10)), ("b", rc(20, 0, 0, 10))]);
        let t = LineText::build(&l);
        assert_eq!(t.span_rect(false, 2, 3), (l.rect, Mapping::Line));
        assert_eq!(t.span_rect(false, 0, 1), (rc(0, 0, 10, 10), Mapping::Word));
        let neg = line(&[("a", rc(-5, 0, 10, 10))]);
        assert_eq!(
            LineText::build(&neg).span_rect(false, 0, 1).1,
            Mapping::Line
        );
        let empty = OcrLine {
            text: "abc".into(),
            rect: rc(1, 2, 3, 4),
            words: Vec::new(),
        };
        let t = LineText::build(&empty);
        assert_eq!(s(&t.spaced.chars), "abc");
        assert_eq!(t.span_rect(false, 0, 2), (rc(1, 2, 3, 4), Mapping::Line));
    }

    #[test]
    fn normalizes_and_folds() {
        assert_eq!(normalize('\u{2014}'), '-');
        assert_eq!(normalize('\u{201C}'), '"');
        assert_eq!(normalize('\u{2019}'), '\'');
        assert_eq!(normalize('\u{00A0}'), ' ');
        assert_eq!(normalize('\u{FF1A}'), ':');
        let folded: String = "Số ĐIỆN THOẠI Liên hệ sđt MẬT KHẨU"
            .chars()
            .map(fold)
            .collect();
        assert_eq!(folded, "so dien thoai lien he sdt mat khau");
    }

    #[test]
    fn wipe_clears() {
        let mut a = String::from("abc");
        wipe(&mut a);
        assert!(a.is_empty());
        let mut v = vec!['x', 'y'];
        wipe_chars(&mut v);
        assert!(v.is_empty());
        let mut b = vec![1u8, 2];
        wipe_bytes(&mut b);
        assert!(b.is_empty());
    }
}
