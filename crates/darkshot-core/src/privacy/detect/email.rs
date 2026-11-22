//! Email: `local@domain.tld`. Cao khi không có khoảng trắng quanh `@`;
//! Trung bình khi OCR tách (tối đa một space mỗi bên).

use super::{Detector, LineCtx, RawMatch};
use crate::privacy::model::{Category, Confidence, RuleId};

pub struct Email;

fn local_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

fn domain_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-')
}

/// Đuôi tệp ảnh/tài nguyên hay gặp sau `@` (`icon@2x.png`) — không phải TLD.
const FILE_EXT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "tif", "tiff",
];

/// Kiểm phần tên miền `chars[s..e)`; trả điểm kết thúc sau khi bỏ `.`/`-` thừa ở cuối.
fn domain_end(chars: &[char], s: usize) -> Option<usize> {
    let mut e = s;
    while e < chars.len() && domain_char(chars[e]) {
        e += 1;
    }
    while e > s && matches!(chars[e - 1], '.' | '-') {
        e -= 1;
    }
    let d = &chars[s..e];
    if d.split(|&c| c == '.').count() < 2 || d.split(|&c| c == '.').any(|l| l.is_empty()) {
        return None;
    }
    let tld = d.split(|&c| c == '.').next_back()?;
    if tld.len() < 2 || !tld.iter().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let is_ext = FILE_EXT.iter().any(|x| {
        x.len() == tld.len() && x.chars().zip(tld).all(|(a, b)| a == b.to_ascii_lowercase())
    });
    if is_ext {
        return None;
    }
    Some(e)
}

/// Khe giữa hai khung từ dưới mức này (phần trăm chiều cao dòng) coi như OCR chèn space vào giữa
/// một từ — cùng ngưỡng ghép từ bị tách của `text::LineText`.
const SPLIT_GAP_PCT: i64 = 35;

/// Phần trước `@` bắt đầu ở `ls`, ngay sau một space; trước đó là một từ trọn chỉ gồm chữ thường,
/// số, `._%+-` (≥ 2 ký tự, không bắt đầu bằng `.`), cách từ sau một khe nhỏ: OCR đã tách phần tên
/// email (Task 136). Trả vị trí đầu từ đó.
fn split_local_start(ctx: &LineCtx<'_>, c: &[char], ls: usize) -> Option<usize> {
    if ls < 3 || c[ls - 1] != ' ' {
        return None;
    }
    let pe = ls - 1;
    let mut ps = pe;
    while ps > 0 && local_char(c[ps - 1]) && !c[ps - 1].is_ascii_uppercase() && pe - ps < 64 {
        ps -= 1;
    }
    if pe - ps < 2 || c[ps] == '.' || (ps > 0 && c[ps - 1] != ' ') {
        return None;
    }
    let (gap, h) = ctx.word_gap(pe, ls)?;
    (gap * 100 < SPLIT_GAP_PCT * h).then_some(ps)
}

impl Detector for Email {
    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        for at in 0..c.len() {
            if c[at] != '@' {
                continue;
            }
            // Dạng OCR tách: đúng một space ở CẢ hai phía `@` (`@user.name` không phải email).
            let before = at > 0 && c[at - 1] == ' ';
            let after = c.get(at + 1) == Some(&' ');
            if before != after {
                continue;
            }
            let spaced = before;
            // phần trước
            let le = if spaced { at - 1 } else { at };
            let mut ls = le;
            while ls > 0 && local_char(c[ls - 1]) && le - ls < 64 {
                ls -= 1;
            }
            while ls < le && c[ls] == '.' {
                ls += 1;
            }
            if ls == le || (ls > 0 && local_char(c[ls - 1])) || c[le - 1] == '.' {
                continue;
            }
            // phần sau
            let ds = if spaced { at + 2 } else { at + 1 };
            if ds >= c.len() || !c[ds].is_ascii_alphanumeric() {
                continue;
            }
            let Some(de) = domain_end(c, ds) else {
                continue;
            };
            let (mut conf, mut rule) = if spaced {
                (Confidence::Medium, RuleId::EmailSpaced)
            } else {
                (Confidence::High, RuleId::EmailPlain)
            };
            let mut start = ls;
            if let Some(ps) = split_local_start(ctx, c, ls) {
                start = ps;
                (conf, rule) = (Confidence::Medium, RuleId::EmailSpaced);
            }
            out.push(RawMatch::one(
                ctx.span(start, de),
                Category::Email,
                conf,
                rule,
            ));
        }
    }
}
