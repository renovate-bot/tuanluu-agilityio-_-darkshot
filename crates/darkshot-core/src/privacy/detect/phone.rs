//! Số điện thoại: VN di động/cố định, quốc tế có `+` (E.164), hoặc 9–12 số có nhãn
//! ở cùng dòng hay dòng trên. Bỏ ngày, giờ, IP, số phiên bản, số tiền, số liền chữ.
//!
//! Một chuỗi số được tách thành các nhóm (cách bằng space `.` `-`, cho phép ngoặc);
//! bộ dò thử các đoạn con liên tiếp của danh sách nhóm, dài nhất trước.

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

/// Số nhóm tối đa của một số điện thoại.
const MAX_GROUPS: usize = 6;
/// Số chữ số tối đa (E.164).
const MAX_DIGITS: usize = 15;

fn is_sep(c: char) -> bool {
    matches!(c, ' ' | '.' | '-')
}

/// Một nhóm: `[s, e)` gồm cả ngoặc và `+`; chữ số ở `[ds, de)`.
#[derive(Clone, Copy)]
struct Group {
    s: usize,
    e: usize,
    ds: usize,
    de: usize,
    plus: bool,
}

impl Group {
    fn len(&self) -> usize {
        self.de - self.ds
    }
}

/// Đọc các nhóm số bắt đầu tại `i`. `seps[k]` là ký tự giữa nhóm k và k+1.
fn read_groups(c: &[char], i: usize) -> (Vec<Group>, Vec<char>) {
    let mut groups: Vec<Group> = Vec::new();
    let mut seps = Vec::new();
    let mut j = i;
    let digit = |p: usize| c.get(p).is_some_and(|c| c.is_ascii_digit());
    loop {
        let s = j;
        let mut k = j;
        let paren = c.get(k) == Some(&'(');
        if paren {
            k += 1;
        }
        let plus = groups.is_empty() && c.get(k) == Some(&'+');
        if plus {
            k += 1;
        }
        let ds = k;
        while digit(k) {
            k += 1;
        }
        let de = k;
        if de == ds {
            break;
        }
        if paren && c.get(k) == Some(&')') {
            k += 1;
        }
        groups.push(Group {
            s,
            e: k,
            ds,
            de,
            plus,
        });
        let next_ok = |p: usize| digit(p) || (c.get(p) == Some(&'(') && digit(p + 1));
        match c.get(k) {
            Some(&x) if is_sep(x) && next_ok(k + 1) => {
                seps.push(x);
                j = k + 1;
            }
            // nhóm kế ngay sau `)`: "(028)38221234" (ký tự cách ')' không phải space nên
            // không tách cửa sổ ở đây)
            _ if paren && c[k - 1] == ')' && next_ok(k) => {
                seps.push(')');
                j = k;
            }
            // số 0 trung kế ngay sau mã nước: "+84(0)912345678"
            _ if plus && groups.len() == 1 && trunk_at(c, k) && digit(k + 3) => {
                seps.push('(');
                j = k;
            }
            _ => break,
        }
    }
    // "(0)" ngay sau nhóm `+mã nước` là số trung kế: không tính chữ số của nó.
    if groups.len() >= 2 && groups[0].plus && trunk_at(c, groups[1].s) {
        groups[1].de = groups[1].ds;
    }
    (groups, seps)
}

/// `(0)` tại `k`.
fn trunk_at(c: &[char], k: usize) -> bool {
    c.get(k..k + 3) == Some(&['(', '0', ')'])
}

fn value(c: &[char], g: &Group) -> u32 {
    c[g.ds..g.de].iter().fold(0u32, |a, d| {
        a.saturating_mul(10)
            .saturating_add(d.to_digit(10).unwrap_or(0))
    })
}

/// Đánh dấu nhóm thuộc ngày (`dd mm yyyy`, `mm dd yyyy`, `yyyy mm dd`) hoặc địa chỉ IPv4.
fn mark_excluded(c: &[char], g: &[Group], seps: &[char]) -> Vec<bool> {
    let mut ex = vec![false; g.len()];
    let year = |x: &Group| x.len() == 4 && (1900..=2099).contains(&value(c, x));
    let day = |x: &Group| (1..=2).contains(&x.len()) && (1..=31).contains(&value(c, x));
    let month = |x: &Group| (1..=2).contains(&x.len()) && (1..=12).contains(&value(c, x));
    for k in 0..g.len().saturating_sub(2) {
        if seps[k] != seps[k + 1] {
            continue;
        }
        let (a, b, d) = (&g[k], &g[k + 1], &g[k + 2]);
        let date = (day(a) && month(b) && year(d))
            || (month(a) && day(b) && year(d))
            || (year(a) && month(b) && day(d));
        if date {
            ex[k..k + 3].fill(true);
        }
    }
    for k in 0..g.len().saturating_sub(3) {
        let dots = seps[k..k + 3].iter().all(|&s| s == '.');
        if dots && g[k..k + 4].iter().all(|x| x.len() <= 3) {
            ex[k..k + 4].fill(true);
        }
    }
    ex
}

/// Số chữ số liền nhau ngay trước `p` (không gồm `p`).
fn digits_before(c: &[char], p: usize) -> usize {
    c[..p]
        .iter()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .count()
}

fn digits_after(c: &[char], p: usize) -> usize {
    c[p.min(c.len())..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count()
}

/// Ký tự trước nhóm đầu gợi ý giờ, ngày, phân số, số tiền, số thập phân.
/// `:` chỉ là giờ khi hai phía là nhóm 1–2 chữ số; `/` chỉ là ngày/phân số khi nhóm kề ≤ 4 chữ số.
fn bad_before(c: &[char], first: &Group) -> bool {
    let Some(p) = first.s.checked_sub(1) else {
        return false;
    };
    let digit_before = p > 0 && c[p - 1].is_ascii_digit();
    match c[p] {
        x if x.is_alphanumeric() => true,
        '$' | '\u{20AC}' | '\u{00A3}' | '_' | '#' => true,
        ':' => digit_before && digits_before(c, p) <= 2 && first.len() <= 2,
        '/' => digit_before && (digits_before(c, p) <= 4 || first.len() <= 4),
        ',' | '.' => digit_before,
        _ => false,
    }
}

fn bad_after(c: &[char], last: &Group) -> bool {
    let p = last.e;
    let Some(&x) = c.get(p) else {
        return false;
    };
    let digit_after = c.get(p + 1).is_some_and(|c| c.is_ascii_digit());
    let bad = match x {
        x if x.is_alphanumeric() => true,
        '$' | '%' | '\u{20AC}' | '\u{20AB}' | '_' | '@' => true,
        ':' => digit_after && digits_after(c, p + 1) <= 2 && last.len() <= 2,
        '/' => digit_after && (digits_after(c, p + 1) <= 4 || last.len() <= 4),
        ',' | '.' => digit_after,
        _ => false,
    };
    bad || money_after(c, p)
}

/// Đơn vị tiền sau space: "500 000 đ", "100 VND", "20 USD".
fn money_after(c: &[char], p: usize) -> bool {
    let mut k = p;
    while c.get(k) == Some(&' ') {
        k += 1;
    }
    if k == p {
        return false;
    }
    let f = |i: usize| c.get(i).map(|&x| crate::privacy::text::fold(x));
    let (n0, n1, n2) = (f(k), f(k + 1), f(k + 2));
    (n0 == Some('d') && n1.is_none_or(|c| !c.is_alphanumeric()))
        || (n0 == Some('v') && n1 == Some('n') && n2 == Some('d'))
        || (n0 == Some('u') && n1 == Some('s') && n2 == Some('d'))
        || matches!(n0, Some('$' | '\u{20AC}' | '\u{20AB}'))
}

/// Số phiên bản: nhóm ngắn (≤ 3 chữ số) cách bằng `.`, tổng < 9 chữ số.
fn version_like(win: &[Group], seps: &[char]) -> bool {
    win.len() >= 2
        && seps.iter().all(|&s| s == '.')
        && win.iter().all(|g| g.len() <= 3)
        && win.iter().map(Group::len).sum::<usize>() < 9
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

/// Phân loại một cửa sổ nhóm: luật mạnh (`labeled == false`) hoặc chỉ luật có nhãn.
fn classify(c: &[char], win: &[Group], labeled: bool) -> Option<(Confidence, RuleId)> {
    let plus = win[0].plus;
    let mut buf = [0u8; MAX_DIGITS];
    let mut n = 0;
    for g in win {
        for x in &c[g.ds..g.de] {
            buf[n] = x.to_digit(10).unwrap_or(0) as u8;
            n += 1;
        }
    }
    let d = &buf[..n];
    let hit = if labeled {
        (!plus && (9..=12).contains(&n)).then_some((Confidence::Medium, RuleId::PhoneLabeled))
    } else if vn_mobile(d, plus) {
        Some((Confidence::High, RuleId::PhoneVnMobile))
    } else if vn_fixed(d, plus) {
        let conf = if plus {
            Confidence::High
        } else {
            Confidence::Medium
        };
        Some((conf, RuleId::PhoneVnFixed))
    } else if plus && (8..=15).contains(&n) {
        Some((Confidence::High, RuleId::PhoneIntl))
    } else {
        None
    };
    buf.fill(0);
    hit
}

impl Detector for Phone {
    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        let labeled = has_label(ctx.folded(), LABELS)
            || ctx.prev_folded().is_some_and(|p| has_label(p, LABELS));
        let digit = |p: usize| c.get(p).is_some_and(|c| c.is_ascii_digit());
        let mut i = 0;
        while i < c.len() {
            let starts = digit(i)
                || (c[i] == '+' && digit(i + 1))
                || (c[i] == '(' && (digit(i + 1) || (c.get(i + 1) == Some(&'+') && digit(i + 2))));
            if !starts || (i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '+')) {
                i += 1;
                continue;
            }
            let (g, seps) = read_groups(c, i);
            let Some(last) = g.last() else {
                i += 1;
                continue;
            };
            i = last.e.max(i + 1);
            let ex = mark_excluded(c, &g, &seps);
            // Cửa sổ `[a, b)` dùng được: không chứa nhóm bị loại; biên trong chuỗi phải là space,
            // biên chuỗi không gợi ý giờ/ngày/tiền; không phải số phiên bản.
            let usable = |a: usize, b: usize| {
                if ex[a..b].iter().any(|&x| x) {
                    return false;
                }
                let left_ok = if a == 0 {
                    !bad_before(c, &g[0])
                } else {
                    seps[a - 1] == ' '
                };
                let right_ok = if b == g.len() {
                    !bad_after(c, &g[b - 1])
                } else {
                    seps[b - 1] == ' '
                };
                left_ok && right_ok && !version_like(&g[a..b], &seps[a..b - 1])
            };
            // Lượt 1: luật mạnh; lượt 2: luật có nhãn trên các nhóm chưa dùng.
            let mut used = vec![false; g.len()];
            for pass_labeled in [false, true] {
                if pass_labeled && !labeled {
                    break;
                }
                let mut a = 0;
                while a < g.len() {
                    let mut total = 0;
                    let mut b_max = a;
                    while b_max < g.len() && b_max - a < MAX_GROUPS && !used[b_max] {
                        total += g[b_max].len();
                        if total > MAX_DIGITS {
                            break;
                        }
                        b_max += 1;
                    }
                    let hit = (a + 1..=b_max).rev().find_map(|b| {
                        if !usable(a, b) {
                            return None;
                        }
                        classify(c, &g[a..b], pass_labeled).map(|r| (b, r))
                    });
                    match hit {
                        Some((b, (conf, rule))) => {
                            used[a..b].fill(true);
                            out.push(RawMatch::one(
                                ctx.span(g[a].s, g[b - 1].e),
                                Category::Phone,
                                conf,
                                rule,
                            ));
                            a = b;
                        }
                        None => a += 1,
                    }
                }
            }
        }
    }
}
