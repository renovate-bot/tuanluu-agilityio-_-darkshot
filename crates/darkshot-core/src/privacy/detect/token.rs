//! JWT / token truy cập: JWT giải mã được (Cao), `Bearer <token>` (Cao),
//! nhãn token + giá trị ≥ 16 (Trung bình). Bản giải mã chỉ nằm trong biến cục bộ và bị xóa ngay.

use super::{
    after_separator, find_labels, has_label, letters_and_digits, run_len, starts_with,
    strong_value, Detector, LineCtx, RawMatch,
};
use crate::privacy::model::{Category, Confidence, RuleId};
use crate::privacy::text::wipe_bytes;

pub struct Token;

const LABELS: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "token",
    "bearer",
];

fn b64url(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn bearer_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+' | '/' | '=' | '~')
}

/// Giải mã base64url (không cần đệm). Ký tự lạ → None.
fn decode_b64url(s: &[char]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for &c in s {
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '-' | '+' => 62,
            '_' | '/' => 63,
            _ => {
                wipe_bytes(&mut out);
                return None;
            }
        };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Đoạn đầu JWT giải mã ra JSON bắt đầu `{` và có `"alg"` hoặc `"typ"`.
fn header_ok(seg: &[char]) -> bool {
    let Some(mut d) = decode_b64url(seg) else {
        return false;
    };
    let first = d.iter().find(|b| !b.is_ascii_whitespace()).copied();
    let has = |k: &[u8]| d.windows(k.len()).any(|w| w == k);
    let ok = first == Some(b'{') && (has(b"\"alg\"") || has(b"\"typ\""));
    wipe_bytes(&mut d);
    ok
}

/// JWT bắt đầu tại `i`; trả điểm kết thúc.
fn jwt_at(c: &[char], i: usize) -> Option<usize> {
    if !starts_with(c, i, "eyJ") || (i > 0 && b64url(c[i - 1])) {
        return None;
    }
    let a = run_len(c, i, b64url);
    if a < 8 || c.get(i + a) != Some(&'.') {
        return None;
    }
    let s2 = i + a + 1;
    let b = run_len(c, s2, b64url);
    if b < 8 || c.get(s2 + b) != Some(&'.') {
        return None;
    }
    let s3 = s2 + b + 1;
    let k = run_len(c, s3, b64url);
    if (1..8).contains(&k) {
        return None;
    }
    if !header_ok(&c[i..i + a]) {
        return None;
    }
    Some(s3 + k)
}

impl Detector for Token {
    fn on_joined(&self) -> bool {
        true
    }

    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        let mut i = 0;
        while i < c.len() {
            match jwt_at(c, i) {
                Some(e) => {
                    out.push(RawMatch::one(
                        ctx.span(i, e),
                        Category::Token,
                        Confidence::High,
                        RuleId::TokenJwt,
                    ));
                    i = e;
                }
                None => i += 1,
            }
        }
        let authorization = has_label(ctx.folded(), &["authorization"]);
        for (ls, le) in find_labels(ctx.folded(), LABELS) {
            let is_bearer = ctx.folded()[ls..le].first() == Some(&'b');
            if is_bearer && c.get(le) == Some(&' ') {
                let vs = le + run_len(c, le, |x| x == ' ');
                let n = run_len(c, vs, bearer_char);
                // Không có `Authorization:` thì token phải có cả chữ và số (JWT có luật riêng).
                let v = &c[vs..vs + n];
                if n >= 16 && (authorization || letters_and_digits(v)) {
                    out.push(RawMatch::one(
                        ctx.span(vs, vs + n),
                        Category::Token,
                        Confidence::High,
                        RuleId::TokenBearer,
                    ));
                    continue;
                }
            }
            let Some(vs) = after_separator(c, le) else {
                continue;
            };
            let n = run_len(c, vs, bearer_char);
            if n >= 16 && strong_value(&c[vs..vs + n]) {
                out.push(RawMatch::one(
                    ctx.span(vs, vs + n),
                    Category::Token,
                    Confidence::Medium,
                    RuleId::TokenLabeled,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn decodes_base64url() {
        assert_eq!(
            decode_b64url(&ch("eyJhbGciOiJIUzI1NiJ9")).unwrap(),
            b"{\"alg\":\"HS256\"}"
        );
        assert_eq!(decode_b64url(&ch("TWE")).unwrap(), b"Ma");
        assert!(decode_b64url(&ch("ab$c")).is_none());
        assert!(header_ok(&ch("eyJhbGciOiJIUzI1NiJ9")));
        assert!(!header_ok(&ch("eyJxxxxxxxxxx")));
    }
}
