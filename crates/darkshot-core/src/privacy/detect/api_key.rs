//! Khóa API: tiền tố đã biết + bộ ký tự + độ dài (Cao), hoặc nhãn + giá trị đủ dài (Trung bình).

use super::{
    after_separator, find_labels, mixed, run_len, starts_with, strong_value, Detector, LineCtx,
    RawMatch,
};
use crate::privacy::model::{Category, Confidence, RuleId};
use crate::privacy::text::{is_token_char, wipe_chars};

pub struct ApiKey;

fn upper_digit(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}
fn alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}
fn alnum_us(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
fn alnum_us_dash(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}
fn alnum_dash(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// Độ dài thân: đúng `n` (`exact`) hoặc ≥ `n`.
#[derive(Clone, Copy)]
enum Len {
    Exact(usize),
    AtLeast(usize),
}

struct Prefix {
    p: &'static str,
    body: fn(char) -> bool,
    len: Len,
    rule: RuleId,
}

impl Prefix {
    /// Khóa có chữ hoa `I` mà OCR hay đọc thành `l`/`|` (AWS, GitHub `gh?_`): thử thêm trên bản
    /// sao đã đổi `l`/`|` → `I`.
    fn ocr_i(&self) -> bool {
        matches!(self.rule, RuleId::ApiKeyAws)
            || (self.rule == RuleId::ApiKeyGithub && self.p.len() == 4)
    }
}

const PREFIXES: &[Prefix] = &[
    Prefix {
        p: "AKIA",
        body: upper_digit,
        len: Len::Exact(16),
        rule: RuleId::ApiKeyAws,
    },
    Prefix {
        p: "ASIA",
        body: upper_digit,
        len: Len::Exact(16),
        rule: RuleId::ApiKeyAws,
    },
    Prefix {
        p: "AIza",
        body: alnum_us_dash,
        len: Len::Exact(35),
        rule: RuleId::ApiKeyGoogle,
    },
    Prefix {
        p: "ghp_",
        body: alnum,
        len: Len::Exact(36),
        rule: RuleId::ApiKeyGithub,
    },
    Prefix {
        p: "gho_",
        body: alnum,
        len: Len::Exact(36),
        rule: RuleId::ApiKeyGithub,
    },
    Prefix {
        p: "ghu_",
        body: alnum,
        len: Len::Exact(36),
        rule: RuleId::ApiKeyGithub,
    },
    Prefix {
        p: "ghs_",
        body: alnum,
        len: Len::Exact(36),
        rule: RuleId::ApiKeyGithub,
    },
    Prefix {
        p: "ghr_",
        body: alnum,
        len: Len::Exact(36),
        rule: RuleId::ApiKeyGithub,
    },
    Prefix {
        p: "github_pat_",
        body: alnum_us,
        len: Len::AtLeast(22),
        rule: RuleId::ApiKeyGithub,
    },
    Prefix {
        p: "glpat-",
        body: alnum_us_dash,
        len: Len::AtLeast(20),
        rule: RuleId::ApiKeyGitlab,
    },
    Prefix {
        p: "xoxa-",
        body: alnum_dash,
        len: Len::AtLeast(10),
        rule: RuleId::ApiKeySlack,
    },
    Prefix {
        p: "xoxb-",
        body: alnum_dash,
        len: Len::AtLeast(10),
        rule: RuleId::ApiKeySlack,
    },
    Prefix {
        p: "xoxp-",
        body: alnum_dash,
        len: Len::AtLeast(10),
        rule: RuleId::ApiKeySlack,
    },
    Prefix {
        p: "xoxr-",
        body: alnum_dash,
        len: Len::AtLeast(10),
        rule: RuleId::ApiKeySlack,
    },
    Prefix {
        p: "sk-ant-",
        body: alnum_us_dash,
        len: Len::AtLeast(20),
        rule: RuleId::ApiKeyOpenAi,
    },
    Prefix {
        p: "sk-proj-",
        body: alnum_us_dash,
        len: Len::AtLeast(20),
        rule: RuleId::ApiKeyOpenAi,
    },
    Prefix {
        p: "sk-",
        body: alnum_us_dash,
        len: Len::AtLeast(20),
        rule: RuleId::ApiKeyOpenAi,
    },
    Prefix {
        p: "sk_live_",
        body: alnum,
        len: Len::AtLeast(16),
        rule: RuleId::ApiKeyStripe,
    },
    Prefix {
        p: "sk_test_",
        body: alnum,
        len: Len::AtLeast(16),
        rule: RuleId::ApiKeyStripe,
    },
    Prefix {
        p: "rk_live_",
        body: alnum,
        len: Len::AtLeast(16),
        rule: RuleId::ApiKeyStripe,
    },
    Prefix {
        p: "pk_live_",
        body: alnum,
        len: Len::AtLeast(16),
        rule: RuleId::ApiKeyStripe,
    },
    Prefix {
        p: "npm_",
        body: alnum,
        len: Len::Exact(36),
        rule: RuleId::ApiKeyNpm,
    },
];

const LABELS: &[&str] = &[
    "api_key",
    "api-key",
    "apikey",
    "x-api-key",
    "client_secret",
    "secret_key",
    "access_key",
];

fn labeled_value_char(c: char) -> bool {
    is_token_char(c)
}

/// Thân SendGrid `SG.` + 22 + `.` + 43; trả độ dài sau tiền tố.
fn sendgrid(c: &[char], i: usize) -> Option<usize> {
    let a = run_len(c, i, alnum_us_dash);
    if a != 22 || c.get(i + a) != Some(&'.') {
        return None;
    }
    let b = run_len(c, i + a + 1, alnum_us_dash);
    (b == 43).then_some(a + 1 + b)
}

/// Khóa có tiền tố bắt đầu tại `i` trên `c`: vị trí cuối và luật. `only_ocr_i`: chỉ các tiền tố
/// `ocr_i` (lượt trên bản sao đã đổi `l`/`|` → `I`). Biên phải đo trên `orig` (chữ thật).
fn prefixed_at(c: &[char], orig: &[char], i: usize, only_ocr_i: bool) -> Option<(usize, RuleId)> {
    for p in PREFIXES {
        if (only_ocr_i && !p.ocr_i()) || !starts_with(c, i, p.p) {
            continue;
        }
        let s = i + p.p.chars().count();
        let n = run_len(c, s, p.body);
        let ok = match p.len {
            Len::Exact(k) => n == k,
            Len::AtLeast(k) => n >= k,
        };
        let e = s + n;
        if ok && orig.get(e).is_none_or(|&x| !alnum_us(x)) {
            return Some((e, p.rule));
        }
    }
    None
}

/// Bản sao cục bộ: `l`/`|` → `I` (nhầm OCR ở vị trí chữ hoa). Bên gọi ghi đè bản sao khi xong.
fn ocr_i_copy(c: &[char]) -> Option<Vec<char>> {
    if !c.iter().any(|&x| x == 'l' || x == '|') {
        return None;
    }
    let mut v = Vec::with_capacity(c.len());
    v.extend(
        c.iter()
            .map(|&x| if x == 'l' || x == '|' { 'I' } else { x }),
    );
    Some(v)
}

impl Detector for ApiKey {
    fn on_joined(&self) -> bool {
        true
    }

    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        let mut fixed = ocr_i_copy(c);
        let mut i = 0;
        while i < c.len() {
            if i > 0 && alnum_us_dash(c[i - 1]) {
                i += 1;
                continue;
            }
            let mut hit = None;
            if starts_with(c, i, "SG.") {
                if let Some(n) = sendgrid(c, i + 3) {
                    let e = i + 3 + n;
                    if c.get(e).is_none_or(|&x| !alnum_us_dash(x)) {
                        hit = Some((e, RuleId::ApiKeySendgrid));
                    }
                }
            }
            if hit.is_none() {
                hit = prefixed_at(c, c, i, false);
            }
            if hit.is_none() {
                if let Some(f) = &fixed {
                    hit = prefixed_at(f, c, i, true);
                }
            }
            match hit {
                Some((e, rule)) => {
                    out.push(RawMatch::one(
                        ctx.span(i, e),
                        Category::ApiKey,
                        Confidence::High,
                        rule,
                    ));
                    i = e;
                }
                None => i += 1,
            }
        }
        if let Some(f) = &mut fixed {
            wipe_chars(f);
        }
        for (_, le) in find_labels(ctx.folded(), LABELS) {
            let Some(vs) = after_separator(c, le) else {
                continue;
            };
            let n = run_len(c, vs, labeled_value_char);
            let v = &c[vs..vs + n];
            if n >= 16 && mixed(v) && (!ctx.joined || strong_value(v)) {
                out.push(RawMatch::one(
                    ctx.span(vs, vs + n),
                    Category::ApiKey,
                    Confidence::Medium,
                    RuleId::ApiKeyLabeled,
                ));
            }
        }
    }
}
