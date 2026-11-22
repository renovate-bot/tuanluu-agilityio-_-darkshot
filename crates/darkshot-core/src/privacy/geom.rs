//! Hình học phát hiện: khoảng ký tự → khung, ưu tiên khi chồng nhau, gộp trùng,
//! ghép phát hiện giữa các dải.

use super::detect::RawMatch;
use super::model::{Confidence, Finding, Mapping};
use super::text::LineText;
use crate::geometry::Rect;

/// Đổi trùng khớp thô thành phát hiện (khung px ảnh gốc, `id` tạm 0).
pub(crate) fn to_finding(lines: &[LineText], m: &RawMatch) -> Finding {
    let mut rects = Vec::with_capacity(m.spans.len());
    let mut mapping = Mapping::Word;
    for s in &m.spans {
        let (r, mp) = lines[s.line].span_rect(s.joined, s.start, s.end);
        if mp == Mapping::Line {
            mapping = Mapping::Line;
        }
        rects.push(r);
    }
    dedup_rects(&mut rects);
    Finding {
        id: 0,
        category: m.category,
        confidence: m.confidence,
        rects,
        rule: m.rule,
        mapping,
    }
}

/// Khung `a` chứa trọn `b`.
fn contains(a: Rect, b: Rect) -> bool {
    b.x >= a.x && b.y >= a.y && b.right() <= a.right() && b.bottom() <= a.bottom()
}

/// Hai khung giao nhau với phần chung có diện tích dương.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// Bỏ khung trùng hoặc nằm trọn trong khung khác.
fn dedup_rects(v: &mut Vec<Rect>) {
    let mut out: Vec<Rect> = Vec::with_capacity(v.len());
    for &r in v.iter() {
        if out.iter().any(|&o| contains(o, r)) {
            continue;
        }
        out.retain(|&o| !contains(r, o));
        out.push(r);
    }
    *v = out;
}

/// Mọi khung của `b` nằm trọn trong một khung của `a`.
fn covered_by(a: &Finding, b: &Finding) -> bool {
    b.rects
        .iter()
        .all(|&r| a.rects.iter().any(|&o| contains(o, r)))
}

fn touches(a: &Finding, b: &Finding) -> bool {
    a.rects
        .iter()
        .any(|&r| b.rects.iter().any(|&o| overlaps(r, o)))
}

/// Gộp `b` vào `a` (cùng loại).
fn absorb(a: &mut Finding, b: Finding) {
    if a.confidence == Confidence::Medium && b.confidence == Confidence::High {
        a.confidence = Confidence::High;
        a.rule = b.rule;
    }
    if b.mapping == Mapping::Line {
        a.mapping = Mapping::Line;
    }
    a.rects.extend(b.rects);
    dedup_rects(&mut a.rects);
}

/// Chuẩn hóa danh sách: bỏ phát hiện ưu tiên thấp bị phát hiện ưu tiên cao hơn phủ trọn;
/// gộp phát hiện cùng loại có khung giao nhau; sắp theo vị trí; đánh `id` 0..n.
pub(crate) fn normalize(v: &mut Vec<Finding>) {
    // Ưu tiên: cao trước để phát hiện thấp so được với mọi phát hiện cao.
    v.sort_by_key(|f| f.category.priority());
    let mut out: Vec<Finding> = Vec::with_capacity(v.len());
    for f in v.drain(..) {
        if out
            .iter()
            .any(|o| o.category.priority() < f.category.priority() && covered_by(o, &f))
        {
            continue;
        }
        out.push(f);
    }
    // Gộp cùng loại tới khi không còn cặp nào giao nhau.
    let mut merged: Vec<Finding> = Vec::with_capacity(out.len());
    for f in out {
        let mut f = f;
        while let Some(k) = merged
            .iter()
            .position(|o| o.category == f.category && touches(o, &f))
        {
            let mut o = merged.swap_remove(k);
            absorb(&mut o, f);
            f = o;
        }
        merged.push(f);
    }
    merged.sort_by_key(|f| {
        let r = f.rects.iter().min_by_key(|r| (r.y, r.x)).copied();
        r.map(|r| (r.y, r.x)).unwrap_or((i32::MAX, i32::MAX))
    });
    for (i, f) in merged.iter_mut().enumerate() {
        f.id = i as u32;
    }
    *v = merged;
}

/// Dịch phát hiện của một dải (độ dời `dy`) và gộp với phát hiện đã có
/// (cùng loại, khung giao nhau). `id` đánh lại 0..n.
pub fn merge_tile(acc: &mut Vec<Finding>, tile: Vec<Finding>, dy: i32) {
    for mut f in tile {
        for r in &mut f.rects {
            r.y = r.y.saturating_add(dy);
        }
        acc.push(f);
    }
    normalize(acc);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::privacy::model::{Category, RuleId};

    fn rc(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn f(cat: Category, conf: Confidence, rects: Vec<Rect>) -> Finding {
        Finding {
            id: 99,
            category: cat,
            confidence: conf,
            rects,
            rule: RuleId::EmailPlain,
            mapping: Mapping::Word,
        }
    }

    #[test]
    fn merge_tile_joins_findings_in_overlap() {
        // Dải 1: y 0..1000; dải 2 bắt đầu ở 872 (chồng 128 px).
        let mut acc = vec![
            f(
                Category::Email,
                Confidence::High,
                vec![rc(10, 900, 200, 20)],
            ),
            f(
                Category::Phone,
                Confidence::High,
                vec![rc(10, 100, 100, 20)],
            ),
        ];
        normalize(&mut acc);
        let tile = vec![
            // cùng email, thấy lại ở y = 28 trong dải 2 (900 - 872), khung lệch nhẹ
            f(
                Category::Email,
                Confidence::Medium,
                vec![rc(12, 29, 200, 20)],
            ),
            // phát hiện mới chỉ trong dải 2
            f(
                Category::Phone,
                Confidence::High,
                vec![rc(10, 500, 100, 20)],
            ),
        ];
        merge_tile(&mut acc, tile, 872);
        assert_eq!(acc.len(), 3);
        let ids: Vec<u32> = acc.iter().map(|f| f.id).collect();
        assert_eq!(ids, vec![0, 1, 2]);
        assert_eq!(acc[0].category, Category::Phone);
        assert_eq!(acc[1].category, Category::Email);
        assert_eq!(acc[1].confidence, Confidence::High);
        assert_eq!(
            acc[1].rects,
            vec![rc(10, 900, 200, 20), rc(12, 901, 200, 20)]
        );
        assert_eq!(acc[2].rects, vec![rc(10, 1372, 100, 20)]);
    }

    #[test]
    fn merge_tile_keeps_different_categories() {
        let mut acc = vec![f(Category::Email, Confidence::High, vec![rc(0, 0, 50, 10)])];
        merge_tile(
            &mut acc,
            vec![f(
                Category::Phone,
                Confidence::High,
                vec![rc(100, 0, 50, 10)],
            )],
            0,
        );
        assert_eq!(acc.len(), 2);
    }

    #[test]
    fn higher_priority_covering_drops_lower() {
        let mut v = vec![
            f(
                Category::Password,
                Confidence::Medium,
                vec![rc(10, 0, 50, 10)],
            ),
            f(Category::Token, Confidence::High, vec![rc(0, 0, 100, 10)]),
        ];
        normalize(&mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].category, Category::Token);
        // không phủ trọn → giữ cả hai
        let mut v = vec![
            f(
                Category::Password,
                Confidence::Medium,
                vec![rc(90, 0, 50, 10)],
            ),
            f(Category::Token, Confidence::High, vec![rc(0, 0, 100, 10)]),
        ];
        normalize(&mut v);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn dedups_contained_rects() {
        let mut v = vec![
            rc(0, 0, 10, 10),
            rc(2, 2, 3, 3),
            rc(0, 0, 10, 10),
            rc(20, 0, 5, 5),
        ];
        dedup_rects(&mut v);
        assert_eq!(v, vec![rc(0, 0, 10, 10), rc(20, 0, 5, 5)]);
    }
}
