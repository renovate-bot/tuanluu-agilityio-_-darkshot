//! Thẻ thanh toán: 13–19 chữ số (nhóm 4, hoặc 4-6-5/4-6-4), Luhn đúng.
//! Cao khi có tiền tố hãng; Trung bình khi có nhãn. Chuẩn hóa nhầm OCR chỉ trong chuỗi số.

use super::{has_label, Detector, LineCtx, RawMatch};
use crate::privacy::model::{Category, Confidence, RuleId};
use crate::privacy::text::wipe_bytes;

pub struct Card;

const LABELS: &[&str] = &["card", "the", "visa", "mastercard", "cvv", "exp"];

/// Nhầm OCR trong chuỗi số: `O/o→0`, `l/I→1`, `S→5`, `B→8`.
fn digit_of(c: char) -> Option<(u8, bool)> {
    match c {
        '0'..='9' => Some((c as u8 - b'0', true)),
        'O' | 'o' => Some((0, false)),
        'l' | 'I' => Some((1, false)),
        'S' => Some((5, false)),
        'B' => Some((8, false)),
        _ => None,
    }
}

/// Một nhóm `[start, end)` gồm chữ số (có thể lẫn chữ nhầm OCR, ít hơn số thật).
struct Group {
    start: usize,
    end: usize,
}

fn read_group(c: &[char], i: usize) -> Option<Group> {
    let mut real = 0;
    let mut fake = 0;
    let mut j = i;
    while let Some((_, is_real)) = c.get(j).and_then(|&x| digit_of(x)) {
        if is_real {
            real += 1;
        } else {
            fake += 1;
        }
        j += 1;
    }
    if j == i || real <= fake || c.get(j).is_some_and(|x| x.is_alphanumeric()) {
        return None;
    }
    Some(Group { start: i, end: j })
}

fn shape_ok(lens: &[usize]) -> bool {
    match lens {
        [one] => (13..=19).contains(one),
        [4, 6, 5] | [4, 6, 4] => true,
        [init @ .., last] => init.iter().all(|&n| n == 4) && (1..=4).contains(last),
        [] => false,
    }
}

pub(crate) fn luhn(d: &[u8]) -> bool {
    let mut sum = 0u32;
    for (k, &x) in d.iter().rev().enumerate() {
        let mut v = u32::from(x);
        if k % 2 == 1 {
            v *= 2;
            if v > 9 {
                v -= 9;
            }
        }
        sum += v;
    }
    sum.is_multiple_of(10)
}

fn brand(d: &[u8]) -> bool {
    let n = |k: usize| d.iter().take(k).fold(0u32, |a, &x| a * 10 + u32::from(x));
    d[0] == 4
        || (51..=55).contains(&n(2))
        || (2221..=2720).contains(&n(4))
        || matches!(n(2), 34 | 37 | 65 | 62)
        || n(4) == 6011
        || (3528..=3589).contains(&n(4))
}

impl Detector for Card {
    fn on_joined(&self) -> bool {
        true
    }

    fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>) {
        let c = ctx.chars();
        let mut i = 0;
        while i < c.len() {
            if digit_of(c[i]).is_none() || (i > 0 && c[i - 1].is_alphanumeric()) {
                i += 1;
                continue;
            }
            let mut groups = Vec::new();
            let mut j = i;
            while let Some(g) = read_group(c, j) {
                j = g.end;
                groups.push(g);
                if matches!(c.get(j), Some(' ' | '-'))
                    && c.get(j + 1).is_some_and(|&x| digit_of(x).is_some())
                {
                    j += 1;
                } else {
                    break;
                }
            }
            if groups.is_empty() {
                i += 1;
                continue;
            }
            let mut next = groups.last().map_or(i + 1, |g| g.end);
            'outer: for a in 0..groups.len() {
                for b in (a + 1..=groups.len()).rev() {
                    let lens: Vec<usize> = groups[a..b].iter().map(|g| g.end - g.start).collect();
                    let total: usize = lens.iter().sum();
                    if !(13..=19).contains(&total) || !shape_ok(&lens) {
                        continue;
                    }
                    let (s, e) = (groups[a].start, groups[b - 1].end);
                    let mut d: Vec<u8> = c[s..e]
                        .iter()
                        .filter_map(|&x| digit_of(x).map(|v| v.0))
                        .collect();
                    let uniform = d.iter().all(|&x| x == d[0]);
                    let hit = if uniform || !luhn(&d) {
                        None
                    } else if brand(&d) {
                        Some((Confidence::High, RuleId::CardBrand))
                    } else if has_label(ctx.folded(), LABELS) {
                        Some((Confidence::Medium, RuleId::CardLabeled))
                    } else {
                        None
                    };
                    wipe_bytes(&mut d);
                    if let Some((conf, rule)) = hit {
                        out.push(RawMatch::one(
                            ctx.span(s, e),
                            Category::CreditCard,
                            conf,
                            rule,
                        ));
                        next = e;
                        break 'outer;
                    }
                }
            }
            i = next.max(i + 1);
        }
    }
}
