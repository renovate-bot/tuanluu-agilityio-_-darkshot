//! Khóa riêng PEM: dòng `-----BEGIN … PRIVATE KEY-----` tới dòng `-----END … PRIVATE KEY-----`;
//! không thấy END thì tới dòng cuối liên tiếp trông như base64. Một phát hiện, mỗi dòng một khung.

use super::{Detector, LineCtx, RawMatch, Span};
use crate::privacy::model::{Category, Confidence, RuleId};
use crate::privacy::text::LineText;

pub struct Pem;

/// Số dòng tối đa tìm END.
const MAX_LINES: usize = 200;

/// Dòng có `-----<word> [A-Z ]*PRIVATE KEY-----` (≥ 1 gạch mỗi phía: OCR hay đọc `——`).
fn marker(c: &[char], word: &str) -> bool {
    let w: Vec<char> = word.chars().collect();
    let key: Vec<char> = "PRIVATE KEY".chars().collect();
    let n = c.len();
    for i in 0..n {
        if c[i] != '-' || (i > 0 && c[i - 1] == '-') {
            continue;
        }
        let d = c[i..].iter().take_while(|&&x| x == '-').count();
        let mut j = i + d;
        while c.get(j) == Some(&' ') {
            j += 1;
        }
        if !c[j.min(n)..].starts_with(&w) {
            continue;
        }
        j += w.len();
        // [A-Z ]* rồi "PRIVATE KEY"
        let rest = &c[j..];
        let mut k = 0;
        while k <= rest.len() {
            if rest[k..].starts_with(&key) {
                let after = &rest[k + key.len()..];
                let mut m = 0;
                while after.get(m) == Some(&' ') {
                    m += 1;
                }
                if after[m..].iter().take_while(|&&x| x == '-').count() >= 1 {
                    return true;
                }
                break;
            }
            match rest.get(k) {
                Some(&x) if x.is_ascii_uppercase() || x == ' ' => k += 1,
                _ => break,
            }
        }
    }
    false
}

/// Dòng (bản ghép từ, bỏ space) trông như base64: ≥ 16 ký tự `[A-Za-z0-9+/=]`.
fn base64_like(t: &LineText) -> bool {
    let c = &t.joined.chars;
    let mut n = 0;
    for &x in c {
        if x == ' ' {
            continue;
        }
        if !(x.is_ascii_alphanumeric() || matches!(x, '+' | '/' | '=')) {
            return false;
        }
        n += 1;
    }
    n >= 16
}

fn whole(lines: &[LineText], i: usize) -> Span {
    Span {
        line: i,
        start: 0,
        end: lines[i].spaced.chars.len(),
        joined: false,
    }
}

impl Detector for Pem {
    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let lines = ctx.lines;
        let i = ctx.index;
        if !marker(&lines[i].spaced.chars, "BEGIN") {
            return;
        }
        let stop = (i + 1 + MAX_LINES).min(lines.len());
        let end = (i + 1..stop).find(|&j| marker(&lines[j].spaced.chars, "END"));
        let last = match end {
            Some(j) => j,
            None => {
                let mut j = i;
                while j + 1 < lines.len() && base64_like(&lines[j + 1]) {
                    j += 1;
                }
                j
            }
        };
        out.push(RawMatch {
            spans: (i..=last).map(|k| whole(lines, k)).collect(),
            category: Category::PrivateKey,
            confidence: Confidence::High,
            rule: RuleId::PemBlock,
        });
    }
}
