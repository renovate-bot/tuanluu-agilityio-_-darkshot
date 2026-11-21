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

/// Tên biến `.env` chứa các chuỗi này ở bất kỳ đâu (`DBPASSWORD`, `PGPASSWORD`).
const ENV_ANYWHERE: &[&str] = &["PASSWORD", "PASSWD", "PWD"];
/// Tên kết thúc bằng các chuỗi này (`APIKEY`, `GITHUBTOKEN`).
const ENV_SUFFIX: &[&str] = &["TOKEN", "APIKEY", "SECRETKEY", "PRIVATEKEY", "ACCESSKEY"];
/// Một đoạn (tách theo `_`) đúng bằng các từ này (`API_KEY`, `GITHUB_TOKEN`).
/// Không nhận `KEYBOARD`, `MONKEY`, `TOKENIZER`.
const ENV_SEGMENT: &[&str] = &["TOKEN", "KEY"];
/// Đoạn cuối `…KEY` có ≥ 3 chữ trước `KEY` (`MASTERKEY`, `SSHKEY`) là bí mật, trừ các từ thường này.
const KEY_WORDS: &[&str] = &["MONKEY", "HOTKEY", "TURKEY", "DONKEY"];
/// Tên chỉ thư mục làm việc của shell: giá trị là đường dẫn tuyệt đối thì không phải mật khẩu.
const PWD_NAMES: &[&str] = &["PWD", "OLDPWD"];

fn eq(a: &str, b: &[char]) -> bool {
    a.len() == b.len() && a.chars().zip(b).all(|(x, &y)| x == y)
}

/// `SECRET`/`SECRETS` là cả đoạn, hoặc `SECRET` theo sau là `_`, `KEY` hay hết tên (không nhận
/// `SECRETARY`, `SECRETSANTA`).
fn has_secret(name: &[char]) -> bool {
    const S: &str = "SECRET";
    (0..name.len().saturating_sub(S.len() - 1)).any(|k| {
        let rest = &name[k + S.len()..];
        eq(S, &name[k..k + S.len()])
            && (rest.is_empty()
                || rest[0] == '_'
                || rest.starts_with(&['K', 'E', 'Y'])
                || rest == ['S']
                || rest.starts_with(&['S', '_']))
    })
}

/// Đoạn cuối kết thúc bằng `KEY` sau ≥ 3 chữ, không phải từ thường (`MONKEY`, `HOTKEY`…).
fn long_key_suffix(name: &[char]) -> bool {
    let last = name.rsplit(|&x| x == '_').next().unwrap_or(name);
    last.len() >= 6 && eq("KEY", &last[last.len() - 3..]) && !KEY_WORDS.iter().any(|w| eq(w, last))
}

/// Tên biến `.env` (chữ hoa, số, `_`) gợi ý bí mật.
fn secret_env_name(name: &[char]) -> bool {
    ENV_ANYWHERE
        .iter()
        .any(|k| name.windows(k.len()).any(|w| eq(k, w)))
        || has_secret(name)
        || ENV_SUFFIX
            .iter()
            .any(|k| name.len() >= k.len() && eq(k, &name[name.len() - k.len()..]))
        || name
            .split(|&x| x == '_')
            .any(|seg| ENV_SEGMENT.iter().any(|k| eq(k, seg)))
        || long_key_suffix(name)
}

/// Đường dẫn tuyệt đối: `/…`, `~/…`, `X:\…`, `X:/…`, `\\máy\…`.
fn absolute_path(v: &[char]) -> bool {
    match v {
        ['/', ..] | ['~', '/' | '\\', ..] | ['\\', '\\', ..] => true,
        [d, ':', '\\' | '/', ..] => d.is_ascii_alphabetic(),
        _ => false,
    }
}

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
        let folded = ctx.folded();
        for (ls, le) in find_labels(folded, LABELS) {
            let Some(vs) = after_separator(c, le) else {
                continue;
            };
            if vs >= c.len() || c[vs] == ' ' {
                continue;
            }
            // `pwd: /home/x` là thư mục làm việc, không phải mật khẩu.
            if eq("pwd", &folded[ls..le]) && absolute_path(&c[vs..]) {
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
            let hit = c.get(i + n) == Some(&'=') && secret_env_name(name);
            if hit {
                let vs = i + n + 1;
                let cwd = PWD_NAMES.iter().any(|k| eq(k, name)) && absolute_path(&c[vs..]);
                if vs < c.len() && c[vs] != ' ' && !cwd {
                    let e = word_end(c, vs);
                    self.push(ctx, out, vs, e, RuleId::PasswordEnv);
                }
            }
            i += n.max(1);
        }
    }
}
