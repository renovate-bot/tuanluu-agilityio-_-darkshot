//! Bộ dò: mỗi loại một tệp, độc lập; đăng ký ở `ALL`.

mod api_key;
mod card;
mod email;
mod password;
mod pem;
mod phone;
mod token;

use super::model::{Category, Confidence, RuleId};
use super::text::{is_combining, LineText};

/// Ngữ cảnh một dòng đang quét. Bộ dò chỉ đọc; không giữ chữ lại.
pub struct LineCtx<'a> {
    /// Mọi dòng của lượt quét (cho luật nhiều dòng như PEM, nhãn dòng trên).
    pub lines: &'a [LineText],
    /// Dòng đang quét.
    pub index: usize,
    /// Đang quét bản ghép từ (true) hay bản có khoảng trắng (false).
    pub joined: bool,
}

impl<'a> LineCtx<'a> {
    pub fn chars(&self) -> &'a [char] {
        &self.lines[self.index].version(self.joined).chars
    }

    pub fn folded(&self) -> &'a [char] {
        &self.lines[self.index].version(self.joined).folded
    }

    /// Bản có khoảng trắng, viết thường bỏ dấu, của dòng trên (nếu có).
    pub fn prev_folded(&self) -> Option<&'a [char]> {
        self.index
            .checked_sub(1)
            .map(|i| self.lines[i].spaced.folded.as_slice())
    }

    /// Khe (px) giữa từ kết thúc ở `prev_end` và từ bắt đầu ở `next_start`, kèm chiều cao dòng.
    pub fn word_gap(&self, prev_end: usize, next_start: usize) -> Option<(i64, i64)> {
        self.lines[self.index].word_gap(self.joined, prev_end, next_start)
    }

    /// Khoảng `[start, end)` trên dòng và bản đang quét.
    pub fn span(&self, start: usize, end: usize) -> Span {
        Span {
            line: self.index,
            start,
            end,
            joined: self.joined,
        }
    }
}

/// Khoảng ký tự trên một dòng, trong bản `joined` hay bản có khoảng trắng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub joined: bool,
}

/// Một trùng khớp thô (chỉ vị trí, không chữ).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawMatch {
    pub spans: Vec<Span>,
    pub category: Category,
    pub confidence: Confidence,
    pub rule: RuleId,
}

impl RawMatch {
    pub fn one(span: Span, category: Category, confidence: Confidence, rule: RuleId) -> Self {
        RawMatch {
            spans: vec![span],
            category,
            confidence,
            rule,
        }
    }
}

/// Một bộ dò.
pub trait Detector {
    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>);
    /// Chạy cả trên bản ghép từ bị OCR tách (Khóa API, JWT/token, Thẻ).
    fn on_joined(&self) -> bool {
        false
    }
}

/// Mọi bộ dò đã đăng ký.
pub static ALL: &[&(dyn Detector + Sync)] = &[
    &pem::Pem,
    &token::Token,
    &api_key::ApiKey,
    &card::Card,
    &password::Password,
    &email::Email,
    &phone::Phone,
];

// ---- tiện ích chung cho các bộ dò ----

/// So `label` (chữ thường; space khớp ≥ 1 space) với `folded` từ `pos`, không phân biệt
/// hoa thường, bỏ qua dấu tổ hợp. Nhãn có dấu thì truyền bản chưa bỏ dấu. Trả vị trí ngay sau nhãn.
pub(crate) fn label_at(folded: &[char], pos: usize, label: &str) -> Option<usize> {
    let mut i = pos;
    for lc in label.chars() {
        while i < folded.len() && i > pos && is_combining(folded[i]) {
            i += 1;
        }
        if lc == ' ' {
            if folded.get(i) != Some(&' ') {
                return None;
            }
            while folded.get(i) == Some(&' ') {
                i += 1;
            }
            continue;
        }
        if folded.get(i).map(|&c| lower(c)) != Some(lc) {
            return None;
        }
        i += 1;
    }
    while i < folded.len() && is_combining(folded[i]) {
        i += 1;
    }
    Some(i)
}

/// Chữ thường một ký tự (giữ dấu); bản đã bỏ dấu không đổi.
fn lower(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || is_combining(c)
}

/// Mọi chỗ có một nhãn trong `labels` đứng riêng (trước/sau không phải chữ/số).
/// Nhãn dài hơn thử trước; trả `(start, end)`.
pub(crate) fn find_labels(folded: &[char], labels: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < folded.len() {
        if i > 0 && word_char(folded[i - 1]) {
            i += 1;
            continue;
        }
        let mut best: Option<usize> = None;
        for l in labels {
            if let Some(e) = label_at(folded, i, l) {
                if e < folded.len() && word_char(folded[e]) {
                    continue;
                }
                if best.is_none_or(|b| e > b) {
                    best = Some(e);
                }
            }
        }
        match best {
            Some(e) => {
                out.push((i, e));
                i = e.max(i + 1);
            }
            None => i += 1,
        }
    }
    out
}

/// Có nhãn nào trong `labels` đứng riêng trong dòng.
pub(crate) fn has_label(folded: &[char], labels: &[&str]) -> bool {
    !find_labels(folded, labels).is_empty()
}

/// Sau nhãn: bỏ space/nháy, cần `:` hoặc `=`, rồi bỏ space/nháy. Trả vị trí đầu giá trị.
pub(crate) fn after_separator(chars: &[char], pos: usize) -> Option<usize> {
    let mut i = pos;
    while matches!(chars.get(i), Some(' ' | '"' | '\'')) {
        i += 1;
    }
    if !matches!(chars.get(i), Some(':' | '=')) {
        return None;
    }
    i += 1;
    while matches!(chars.get(i), Some(' ' | '"' | '\'')) {
        i += 1;
    }
    Some(i)
}

/// Độ dài chuỗi liên tiếp từ `pos` thỏa `f`.
pub(crate) fn run_len(chars: &[char], pos: usize, f: impl Fn(char) -> bool) -> usize {
    chars[pos.min(chars.len())..]
        .iter()
        .take_while(|&&c| f(c))
        .count()
}

/// Có cả chữ cái và chữ số.
pub(crate) fn letters_and_digits(v: &[char]) -> bool {
    v.iter().any(|c| c.is_ascii_alphabetic()) && v.iter().any(|c| c.is_ascii_digit())
}

/// Có ≥ 2 nhóm ký tự trong chữ hoa / chữ thường / số.
pub(crate) fn mixed(v: &[char]) -> bool {
    let u = v.iter().any(|c| c.is_ascii_uppercase());
    let l = v.iter().any(|c| c.is_ascii_lowercase());
    let d = v.iter().any(|c| c.is_ascii_digit());
    u as u8 + l as u8 + d as u8 >= 2
}

/// Chỉ áp trên bản ghép từ: giá trị của luật nhãn + giá trị trông như bí mật: chữ + số, hoặc hoa + thường + ký hiệu.
/// Loại câu chữ thường bị ghép từ ("pleaseenteryour…").
pub(crate) fn strong_value(v: &[char]) -> bool {
    let upper = v.iter().any(|c| c.is_ascii_uppercase());
    let lower = v.iter().any(|c| c.is_ascii_lowercase());
    let symbol = v.iter().any(|c| !c.is_ascii_alphanumeric());
    letters_and_digits(v) || (upper && lower && symbol)
}

/// So khớp đúng chữ (phân biệt hoa thường) tại `pos`.
pub(crate) fn starts_with(chars: &[char], pos: usize, s: &str) -> bool {
    (pos..)
        .zip(s.chars())
        .all(|(i, c)| chars.get(i) == Some(&c))
}

#[cfg(test)]
pub(crate) fn card_luhn_for_tests(d: &[u8]) -> bool {
    card::luhn(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(s: &str) -> Vec<char> {
        s.chars().map(crate::privacy::text::fold).collect()
    }

    #[test]
    fn labels_need_boundaries() {
        assert_eq!(find_labels(&f("Tel: 1"), &["tel"]), vec![(0, 3)]);
        assert!(find_labels(&f("hotel 1"), &["tel"]).is_empty());
        assert!(find_labels(&f("telx 1"), &["tel"]).is_empty());
        assert_eq!(
            find_labels(&f("Số  điện thoại"), &["so dien thoai", "dien thoai"]),
            vec![(0, 14)]
        );
        assert_eq!(find_labels(&f("db_password"), &["password"]), vec![(3, 11)]);
    }

    #[test]
    fn separators() {
        let c: Vec<char> = "pwd : \"x\"".chars().collect();
        assert_eq!(after_separator(&c, 3), Some(7));
        let c: Vec<char> = "pwd x".chars().collect();
        assert_eq!(after_separator(&c, 3), None);
    }
}
