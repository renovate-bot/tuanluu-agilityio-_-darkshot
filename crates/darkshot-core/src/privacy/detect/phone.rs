//! Số điện thoại: VN di động/cố định, quốc tế có `+` (E.164), hoặc 9–12 số có nhãn
//! ở cùng dòng hay dòng trên. Bỏ ngày, giờ, IP, số phiên bản, số tiền, số liền chữ.

use super::{has_label, Detector, LineCtx, RawMatch};
use crate::privacy::model::{Category, Confidence, RuleId};

pub struct Phone;

/// Nhãn (viết thường, bỏ dấu).
const LABELS: &[&str] = &[
    "tel",
    "phone",
    "telephone",
    "mobile",
    "hotline",
    "sdt",
    "dien thoai",
    "so dien thoai",
    "lien he",
    "zalo",
];

fn is_sep(c: char) -> bool {
    matches!(c, ' ' | '.' | '-')
}

/// Một chuỗi số ứng viên: vị trí, có `+`, các nhóm số (độ dài), các ký tự cách.
struct Run {
    start: usize,
    end: usize,
    plus: bool,
    groups: Vec<usize>,
    seps: Vec<char>,
}

/// Đọc chuỗi số bắt đầu tại `i` (`+` hoặc chữ số). Nhóm cách nhau đúng một ký tự cách.
fn read_run(c: &[char], i: usize) -> Run {
    let mut j = i;
    let plus = c[j] == '+';
    if plus {
        j += 1;
    }
    let mut groups = Vec::new();
    let mut seps = Vec::new();
    let mut end = j;
    loop {
        let g = c[j..].iter().take_while(|c| c.is_ascii_digit()).count();
        if g == 0 {
            break;
        }
        groups.push(g);
        j += g;
        end = j;
        if j + 1 < c.len() && is_sep(c[j]) && c[j + 1].is_ascii_digit() {
            seps.push(c[j]);
            j += 1;
        } else {
            break;
        }
    }
    Run {
        start: i,
        end,
        plus,
        groups,
        seps,
    }
}

fn digits(c: &[char], r: &Run) -> Vec<u8> {
    c[r.start..r.end]
        .iter()
        .filter_map(|c| c.to_digit(10).map(|d| d as u8))
        .collect()
}

/// Ngày, IP, phiên bản.
fn looks_like_date_ip_version(r: &Run) -> bool {
    let g = &r.groups;
    let same_sep = r.seps.windows(2).all(|w| w[0] == w[1]);
    let date = matches!(g.as_slice(), [4, 1..=2, 1..=2] | [1..=2, 1..=2, 4]);
    if date && same_sep && r.seps.first() != Some(&' ') {
        return true;
    }
    let all_dots = !r.seps.is_empty() && r.seps.iter().all(|&s| s == '.');
    if all_dots && g.len() == 4 && g.iter().all(|&n| n <= 3) {
        return true; // IPv4
    }
    if all_dots && g.len() >= 3 && g[0] <= 2 {
        return true; // số phiên bản
    }
    false
}

/// Ký tự ngay trước/sau gợi ý số tiền, số thập phân, phân số, giờ.
fn bad_neighbors(c: &[char], r: &Run) -> bool {
    let before = r.start.checked_sub(1).map(|i| c[i]);
    let after = c.get(r.end).copied();
    if let Some(b) = before {
        if b.is_alphanumeric() || matches!(b, '$' | '\u{20AC}' | '\u{00A3}' | '/' | ':' | '_') {
            return true;
        }
        if matches!(b, ',' | '.') && r.start >= 2 && c[r.start - 2].is_ascii_digit() {
            return true;
        }
    }
    if let Some(a) = after {
        if a.is_alphanumeric() || matches!(a, '$' | '%' | '\u{20AC}' | '/' | ':' | '_' | '@') {
            return true;
        }
        if matches!(a, ',' | '.') && c.get(r.end + 1).is_some_and(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    // đơn vị tiền sau một space: "500 000 đ", "100 VND", "20 USD"
    let mut k = r.end;
    while c.get(k) == Some(&' ') {
        k += 1;
    }
    if k > r.end {
        let f = |i: usize| c.get(i).map(|&x| crate::privacy::text::fold(x));
        let (n0, n1, n2) = (f(k), f(k + 1), f(k + 2));
        let money = (n0 == Some('d') && n1.is_none_or(|c| !c.is_alphanumeric()))
            || (n0 == Some('v') && n1 == Some('n') && n2 == Some('d'))
            || (n0 == Some('u') && n1 == Some('s') && n2 == Some('d'))
            || matches!(n0, Some('$' | '\u{20AC}' | '\u{20AB}'));
        if money {
            return true;
        }
    }
    c.get(r.end) == Some(&'\u{20AB}')
}

fn vn_mobile(d: &[u8], plus: bool) -> bool {
    let rest = if d.len() == 11 && d[0] == 8 && d[1] == 4 {
        &d[2..]
    } else if !plus && d.len() == 10 && d[0] == 0 {
        &d[1..]
    } else {
        return false;
    };
    matches!(rest[0], 3 | 5 | 7 | 8 | 9)
}

fn vn_fixed(d: &[u8], plus: bool) -> bool {
    (plus && d.len() == 12 && d[0] == 8 && d[1] == 4 && d[2] == 2)
        || (!plus && d.len() == 11 && d[0] == 0 && d[1] == 2)
}

impl Detector for Phone {
    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        let labeled = || {
            has_label(ctx.folded(), LABELS)
                || ctx.prev_folded().is_some_and(|p| has_label(p, LABELS))
        };
        let mut i = 0;
        while i < c.len() {
            let starts = c[i].is_ascii_digit()
                || (c[i] == '+' && c.get(i + 1).is_some_and(|c| c.is_ascii_digit()));
            if !starts || (i > 0 && (c[i - 1].is_ascii_digit() || c[i - 1] == '+')) {
                i += 1;
                continue;
            }
            let r = read_run(c, i);
            i = r.end.max(i + 1);
            if bad_neighbors(c, &r) || looks_like_date_ip_version(&r) {
                continue;
            }
            let mut d = digits(c, &r);
            let hit = if vn_mobile(&d, r.plus) {
                Some((Confidence::High, RuleId::PhoneVnMobile))
            } else if vn_fixed(&d, r.plus) {
                let conf = if r.plus {
                    Confidence::High
                } else {
                    Confidence::Medium
                };
                Some((conf, RuleId::PhoneVnFixed))
            } else if r.plus && (8..=15).contains(&d.len()) {
                Some((Confidence::High, RuleId::PhoneIntl))
            } else if !r.plus && (9..=12).contains(&d.len()) && labeled() {
                Some((Confidence::Medium, RuleId::PhoneLabeled))
            } else {
                None
            };
            crate::privacy::text::wipe_bytes(&mut d);
            if let Some((conf, rule)) = hit {
                out.push(RawMatch::one(
                    ctx.span(r.start, r.end),
                    Category::Phone,
                    conf,
                    rule,
                ));
            }
        }
    }
}
