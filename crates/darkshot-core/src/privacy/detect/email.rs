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

impl Detector for Email {
    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        for at in 0..c.len() {
            if c[at] != '@' {
                continue;
            }
            let mut spaced = false;
            // phần trước
            let mut le = at;
            if le > 0 && c[le - 1] == ' ' {
                le -= 1;
                spaced = true;
            }
            let mut ls = le;
            while ls > 0 && local_char(c[ls - 1]) && le - ls < 64 {
                ls -= 1;
            }
            while ls < le && c[ls] == '.' {
                ls += 1;
            }
            if ls == le || (ls > 0 && local_char(c[ls - 1])) {
                continue;
            }
            // phần sau
            let mut ds = at + 1;
            if c.get(ds) == Some(&' ') {
                ds += 1;
                spaced = true;
            }
            if ds >= c.len() || !c[ds].is_ascii_alphanumeric() {
                continue;
            }
            let Some(de) = domain_end(c, ds) else {
                continue;
            };
            let (conf, rule) = if spaced {
                (Confidence::Medium, RuleId::EmailSpaced)
            } else {
                (Confidence::High, RuleId::EmailPlain)
            };
            out.push(RawMatch::one(ctx.span(ls, de), Category::Email, conf, rule));
        }
    }
}
