//! Mật khẩu / bí mật có nhãn (luôn Trung bình). Chỉ che giá trị; bỏ giá trị đã ẩn
//! (`****`), nhãn không giá trị, chữ "Password" đứng một mình. Cũng `KEY=value` kiểu `.env`.

use super::{after_separator, find_labels, Detector, LineCtx, RawMatch};
use crate::privacy::model::{Category, Confidence, RuleId};

pub struct Password;

/// Nhãn (viết thường, bỏ dấu).
const LABELS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "pass",
    "mat khau",
    "mk",
    "secret",
    "passphrase",
    "pin",
];

const ENV_NAMES: &[&str] = &["PASSWORD", "SECRET", "TOKEN", "KEY"];

const MAX_SPACED: usize = 64;

fn masked(v: &[char]) -> bool {
    // Rỗng cũng coi như đã ẩn (không có gì để che).
    v.iter()
        .filter(|&&c| c != ' ')
        .all(|&c| matches!(c, '*' | '\u{2022}' | '\u{25CF}' | '\u{00B7}' | '\u{2219}'))
}

fn word_end(c: &[char], s: usize) -> usize {
    s + c[s..].iter().take_while(|&&x| x != ' ').count()
}

/// Bỏ nháy/dấu câu thừa ở cuối giá trị.
fn trim_end(c: &[char], s: usize, mut e: usize) -> usize {
    while e > s && matches!(c[e - 1], '"' | '\'' | ',' | ';') {
        e -= 1;
    }
    e
}

/// Giá trị bắt đầu ở `vs`: một từ, hoặc nhiều từ tới trước nhãn kế (`user:`) nếu tổng ≤ 64.
fn value_end(c: &[char], vs: usize) -> usize {
    let first = word_end(c, vs);
    let mut e = first;
    loop {
        let mut s = e;
        while c.get(s) == Some(&' ') {
            s += 1;
        }
        if s >= c.len() {
            break;
        }
        let we = word_end(c, s);
        if matches!(c[we - 1], ':' | '=') || c[s..we].contains(&'=') {
            break;
        }
        e = we;
    }
    if e - vs <= MAX_SPACED {
        e
    } else {
        first
    }
}

impl Password {
    fn push(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>, s: usize, e: usize, rule: RuleId) {
        let c = ctx.chars();
        let e = trim_end(c, s, e);
        if e <= s || masked(&c[s..e]) {
            return;
        }
        out.push(RawMatch::one(
            ctx.span(s, e),
            Category::Password,
            Confidence::Medium,
            rule,
        ));
    }
}

impl Detector for Password {
    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        for (_, le) in find_labels(ctx.folded(), LABELS) {
            let Some(vs) = after_separator(c, le) else {
                continue;
            };
            if vs >= c.len() || c[vs] == ' ' {
                continue;
            }
            let e = value_end(c, vs);
            self.push(ctx, out, vs, e, RuleId::PasswordLabeled);
        }
        // .env: TEN_IN_HOA=value
        let mut i = 0;
        while i < c.len() {
            if (i > 0 && c[i - 1] != ' ') || !c[i].is_ascii_uppercase() {
                i += 1;
                continue;
            }
            let n = c[i..]
                .iter()
                .take_while(|&&x| x.is_ascii_uppercase() || x.is_ascii_digit() || x == '_')
                .count();
            let name = &c[i..i + n];
            let hit = c.get(i + n) == Some(&'=')
                && ENV_NAMES.iter().any(|k| {
                    let k: Vec<char> = k.chars().collect();
                    name.windows(k.len()).any(|w| w == k.as_slice())
                });
            if hit {
                let vs = i + n + 1;
                if vs < c.len() && c[vs] != ' ' {
                    let e = word_end(c, vs);
                    self.push(ctx, out, vs, e, RuleId::PasswordEnv);
                }
            }
            i += n.max(1);
        }
    }
}
