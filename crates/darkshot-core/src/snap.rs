//! Gợi ý khung cửa sổ/màn hình khi chọn vùng (logic thuần, không phụ thuộc hệ điều hành).

use crate::geometry::{Point, Rect};

pub const MIN_SNAP: i32 = 16;
pub const MAX_WINDOWS: usize = 512;

/// Cắt mỗi khung theo `bounds`; bỏ khung có chiều nào < MIN_SNAP sau khi cắt; bỏ khung trùng hệt khung ngay trước;
/// giữ thứ tự; chỉ lấy tối đa MAX_WINDOWS khung cửa sổ đầu; rồi nối thêm `monitors` (cũng cắt, cũng lọc < MIN_SNAP).
pub fn candidates(windows: &[Rect], monitors: &[Rect], bounds: Rect) -> Vec<Rect> {
    let mut out: Vec<Rect> = Vec::new();
    for w in windows.iter().take(MAX_WINDOWS) {
        if let Some(c) = clip(*w, bounds).filter(big_enough) {
            if out.last() != Some(&c) {
                out.push(c);
            }
        }
    }
    out.extend(
        monitors
            .iter()
            .filter_map(|m| clip(*m, bounds))
            .filter(big_enough),
    );
    out
}

fn big_enough(r: &Rect) -> bool {
    r.w >= MIN_SNAP && r.h >= MIN_SNAP
}

/// Ứng viên đầu tiên chứa `p` (theo `Rect::contains`).
pub fn candidate_at(cands: &[Rect], p: Point) -> Option<Rect> {
    cands.iter().copied().find(|c| c.contains(p))
}

/// Giao hai khung; None khi không giao hoặc rỗng. Dùng i64 để không tràn.
pub fn clip(r: Rect, bounds: Rect) -> Option<Rect> {
    let x0 = i64::from(r.x).max(i64::from(bounds.x));
    let y0 = i64::from(r.y).max(i64::from(bounds.y));
    let x1 = (i64::from(r.x) + i64::from(r.w)).min(i64::from(bounds.x) + i64::from(bounds.w));
    let y1 = (i64::from(r.y) + i64::from(r.h)).min(i64::from(bounds.y) + i64::from(bounds.h));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(Rect {
        x: i32::try_from(x0).ok()?,
        y: i32::try_from(y0).ok()?,
        w: i32::try_from(x1 - x0).ok()?,
        h: i32::try_from(y1 - y0).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    fn p(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    #[test]
    fn clip_partial_inside_outside_touching() {
        let b = r(0, 0, 100, 100);
        assert_eq!(clip(r(50, 50, 100, 100), b), Some(r(50, 50, 50, 50)));
        assert_eq!(clip(r(10, 10, 20, 20), b), Some(r(10, 10, 20, 20)));
        assert_eq!(clip(r(200, 200, 10, 10), b), None);
        assert_eq!(clip(r(100, 0, 10, 10), b), None);
        assert_eq!(clip(r(-10, 0, 10, 10), b), None);
        assert_eq!(clip(r(10, 10, 0, 10), b), None);
    }

    #[test]
    fn clip_negative_coordinates() {
        let b = r(-100, -100, 200, 200);
        assert_eq!(
            clip(r(-150, -150, 100, 100), b),
            Some(r(-100, -100, 50, 50))
        );
    }

    #[test]
    fn clip_does_not_overflow_near_i32_max() {
        let b = r(i32::MAX - 100, i32::MAX - 100, 100, 100);
        let big = r(i32::MAX - 50, i32::MAX - 50, 50, 50);
        assert_eq!(clip(big, b), Some(big));
        let huge = r(-5, -5, i32::MAX, i32::MAX);
        assert_eq!(clip(huge, r(0, 0, 10, 10)), Some(r(0, 0, 10, 10)));
        let over = r(i32::MAX - 50, i32::MAX - 50, 100, 100);
        assert_eq!(clip(over, b), Some(big));
    }

    #[test]
    fn candidates_clip_to_bounds() {
        let b = r(0, 0, 100, 100);
        let out = candidates(&[r(50, 50, 100, 100)], &[], b);
        assert_eq!(out, vec![r(50, 50, 50, 50)]);
    }

    #[test]
    fn candidates_drop_small() {
        let b = r(0, 0, 1000, 1000);
        let out = candidates(
            &[r(0, 0, 15, 100), r(0, 0, 16, 16), r(0, 0, 100, 15)],
            &[],
            b,
        );
        assert_eq!(out, vec![r(0, 0, 16, 16)]);
        // nhỏ sau khi cắt
        let out = candidates(&[r(990, 0, 100, 100)], &[], b);
        assert!(out.is_empty());
        // màn hình nhỏ cũng bị lọc
        assert!(candidates(&[], &[r(0, 0, 10, 10)], b).is_empty());
    }

    #[test]
    fn candidates_drop_consecutive_duplicates_only() {
        let b = r(0, 0, 1000, 1000);
        let a = r(0, 0, 50, 50);
        let c = r(10, 10, 50, 50);
        assert_eq!(candidates(&[a, a, c], &[], b), vec![a, c]);
        assert_eq!(candidates(&[a, c, a], &[], b), vec![a, c, a]);
    }

    #[test]
    fn candidates_monitors_appended_in_order() {
        let b = r(0, 0, 3000, 1000);
        let w = r(10, 10, 100, 100);
        let m1 = r(0, 0, 1000, 1000);
        let m2 = r(1000, 0, 2000, 1000);
        assert_eq!(candidates(&[w], &[m1, m2], b), vec![w, m1, m2]);
    }

    #[test]
    fn candidates_cap_windows_at_512_then_monitors() {
        let b = r(0, 0, 5000, 5000);
        let wins: Vec<Rect> = (0..600).map(|i| r(i, i, 50, 50)).collect();
        let m = r(0, 0, 5000, 5000);
        let out = candidates(&wins, &[m], b);
        assert_eq!(out.len(), MAX_WINDOWS + 1);
        assert_eq!(out[0], wins[0]);
        assert_eq!(out[511], wins[511]);
        assert_eq!(out[512], m);
    }

    #[test]
    fn candidates_drop_windows_outside_bounds() {
        let b = r(0, 0, 100, 100);
        assert!(candidates(&[r(500, 500, 50, 50)], &[], b).is_empty());
    }

    #[test]
    fn candidate_at_first_wins_and_edges() {
        let a = r(0, 0, 100, 100);
        let c = r(50, 50, 100, 100);
        assert_eq!(candidate_at(&[a, c], p(60, 60)), Some(a));
        assert_eq!(candidate_at(&[a, c], p(120, 120)), Some(c));
        // mép trái/trên thuộc khung; mép phải/dưới không (Rect::contains)
        assert_eq!(candidate_at(&[a], p(0, 0)), Some(a));
        assert_eq!(candidate_at(&[a], p(0, 99)), Some(a));
        assert_eq!(candidate_at(&[a], p(100, 50)), None);
        assert_eq!(candidate_at(&[a], p(50, 100)), None);
        assert_eq!(candidate_at(&[a], p(99, 99)), Some(a));
    }

    #[test]
    fn candidate_at_monitor_outside_and_empty() {
        let w = r(0, 0, 100, 100);
        let m = r(0, 0, 1000, 1000);
        assert_eq!(candidate_at(&[w, m], p(500, 500)), Some(m));
        assert_eq!(candidate_at(&[w, m], p(2000, 2000)), None);
        assert_eq!(candidate_at(&[], p(1, 1)), None);
    }
}
