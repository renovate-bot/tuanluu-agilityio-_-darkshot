//! Ghim ảnh: hình học, mức phóng, độ mờ. Hàm thuần, không phụ thuộc hệ điều hành.

use crate::geometry::{Point, Rect};

pub const ZOOM_STEPS: [u16; 16] = [
    10, 25, 33, 50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400,
];
pub const MAX_PINS: usize = 32;
/// Phần trăm.
pub const OPACITY_MIN: u8 = 20;
pub const OPACITY_STEP: u8 = 10;
/// Cỡ hiển thị tối đa mỗi chiều (px).
pub const MAX_SIDE: i32 = 16384;
/// Phần tối thiểu còn trong một màn hình (px).
pub const MIN_VISIBLE: i32 = 32;
/// Ảnh từ Lịch sử chiếm tối đa tỉ lệ này của vùng làm việc.
pub const FIT_PERCENT: i32 = 80;

fn scale(v: u32, zoom: u16) -> i32 {
    let r = (v as u64 * zoom as u64 + 50) / 100;
    r.clamp(1, i32::MAX as u64) as i32
}

/// Cỡ hiển thị của ảnh w×h ở mức `zoom` (%), làm tròn gần nhất, mỗi chiều ≥ 1.
pub fn display_size(w: u32, h: u32, zoom: u16) -> (i32, i32) {
    (scale(w, zoom), scale(h, zoom))
}

/// Diện tích hiển thị tối đa (px²) khi phóng trên 100%.
pub const MAX_AREA: i64 = 40_000_000;

/// Mức `zoom` dùng được cho ảnh w×h: mỗi cạnh hiển thị ≤ MAX_SIDE, và khi phóng trên 100%
/// thì diện tích hiển thị ≤ MAX_AREA.
pub fn zoom_allowed(w: u32, h: u32, zoom: u16) -> bool {
    let (dw, dh) = display_size(w, h, zoom);
    dw <= MAX_SIDE && dh <= MAX_SIDE && (zoom <= 100 || dw as i64 * dh as i64 <= MAX_AREA)
}

/// Bậc kế tiếp lớn hơn trong ZOOM_STEPS mà `zoom_allowed`.
pub fn zoom_in(w: u32, h: u32, zoom: u16) -> Option<u16> {
    let next = ZOOM_STEPS.iter().copied().find(|&s| s > zoom)?;
    zoom_allowed(w, h, next).then_some(next)
}

/// Bậc kế tiếp nhỏ hơn trong ZOOM_STEPS.
pub fn zoom_out(zoom: u16) -> Option<u16> {
    ZOOM_STEPS.iter().rev().copied().find(|&s| s < zoom)
}

/// Khung ảnh mới khi đổi mức, giữ `anchor` đứng yên trên cùng điểm ảnh.
pub fn zoom_about(image_rect: Rect, anchor: Point, img_w: u32, img_h: u32, new_zoom: u16) -> Rect {
    let (nw, nh) = display_size(img_w, img_h, new_zoom);
    let axis = |pos: i32, old: i32, a: i32, new: i32| -> i32 {
        if old <= 0 {
            return pos;
        }
        let off = (a as i128 - pos as i128) * new as i128;
        let den = old as i128;
        let moved = (2 * off + den).div_euclid(2 * den);
        (a as i128 - moved).clamp(i32::MIN as i128, i32::MAX as i128) as i32
    };
    Rect {
        x: axis(image_rect.x, image_rect.w, anchor.x, nw),
        y: axis(image_rect.y, image_rect.h, anchor.y, nh),
        w: nw,
        h: nh,
    }
}

/// Bậc lớn nhất ≤ 100 để ảnh vừa FIT_PERCENT% của `area` mỗi chiều.
pub fn fit_zoom(img_w: u32, img_h: u32, area: Rect) -> u16 {
    let max_w = area.w as i64 * FIT_PERCENT as i64 / 100;
    let max_h = area.h as i64 * FIT_PERCENT as i64 / 100;
    ZOOM_STEPS
        .iter()
        .rev()
        .copied()
        .filter(|&z| z <= 100)
        .find(|&z| {
            let (dw, dh) = display_size(img_w, img_h, z);
            dw as i64 <= max_w && dh as i64 <= max_h
        })
        .unwrap_or(ZOOM_STEPS[0])
}

/// Khung ảnh căn giữa `area` ở mức `zoom`.
pub fn centered(img_w: u32, img_h: u32, zoom: u16, area: Rect) -> Rect {
    let (w, h) = display_size(img_w, img_h, zoom);
    Rect {
        x: center1(area.x, area.w, w),
        y: center1(area.y, area.h, h),
        w,
        h,
    }
}

/// Đầu đoạn dài `s` căn giữa đoạn `[a, a + len)` (làm tròn xuống), kẹp vào i32.
fn center1(a: i32, len: i32, s: i32) -> i32 {
    (a as i64 + (len as i64 - s as i64).div_euclid(2)).clamp(i32::MIN as i64, i32::MAX as i64)
        as i32
}

/// Khung ảnh ban đầu trong `area` ở mức `zoom`: mỗi chiều, cỡ hiển thị ≤ cỡ vùng thì căn giữa như
/// `centered`; lớn hơn thì neo mép trái (ngang) / mép trên (dọc) vào vùng để phần đầu ảnh luôn thấy.
pub fn placed_in(img_w: u32, img_h: u32, zoom: u16, area: Rect) -> Rect {
    let (w, h) = display_size(img_w, img_h, zoom);
    let place = |a: i32, len: i32, s: i32| if s > len { a } else { center1(a, len, s) };
    Rect {
        x: place(area.x, area.w, w),
        y: place(area.y, area.h, h),
        w,
        h,
    }
}

/// Độ dài phần giao của hai đoạn [a, a+la) và [b, b+lb); âm khi không giao.
fn overlap1(a: i32, la: i32, b: i32, lb: i32) -> i64 {
    let lo = (a as i64).max(b as i64);
    let hi = (a as i64 + la as i64).min(b as i64 + lb as i64);
    hi - lo
}

/// Dời `r` ít nhất có thể sao cho đủ MIN_VISIBLE px mỗi chiều nằm trong một màn hình.
pub fn keep_visible(r: Rect, monitors: &[Rect]) -> Rect {
    if monitors.is_empty() {
        return r;
    }
    let need_w = MIN_VISIBLE.min(r.w) as i64;
    let need_h = MIN_VISIBLE.min(r.h) as i64;
    let ok = |m: &Rect| {
        overlap1(r.x, r.w, m.x, m.w) >= need_w.min(m.w as i64)
            && overlap1(r.y, r.h, m.y, m.h) >= need_h.min(m.h as i64)
    };
    if monitors.iter().any(ok) {
        return r;
    }
    let area = |m: &Rect| overlap1(r.x, r.w, m.x, m.w).max(0) * overlap1(r.y, r.h, m.y, m.h).max(0);
    let dist = |m: &Rect| {
        let dx = (-overlap1(r.x, r.w, m.x, m.w)).max(0);
        let dy = (-overlap1(r.y, r.h, m.y, m.h)).max(0);
        dx * dx + dy * dy
    };
    let best_area = monitors.iter().map(area).max().unwrap_or(0);
    let m = if best_area > 0 {
        monitors.iter().find(|m| area(m) == best_area)
    } else {
        monitors.iter().min_by_key(|m| dist(m))
    }
    .copied()
    .unwrap_or(monitors[0]);
    let fix = |pos: i64, len: i64, mpos: i64, mlen: i64, need: i64| -> i32 {
        let need = need.min(mlen);
        let mut p = pos;
        if p + len < mpos + need {
            p = mpos + need - len;
        } else if p > mpos + mlen - need {
            p = mpos + mlen - need;
        }
        p.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    };
    Rect {
        x: fix(r.x as i64, r.w as i64, m.x as i64, m.w as i64, need_w),
        y: fix(r.y as i64, r.h as i64, m.y as i64, m.h as i64, need_h),
        w: r.w,
        h: r.h,
    }
}

/// Độ mờ kế tiếp (phần trăm); `up` là đục hơn. Kẹp [OPACITY_MIN, 100].
pub fn opacity_step(cur: u8, up: bool) -> u8 {
    let cur = cur.clamp(OPACITY_MIN, 100);
    if up {
        cur.saturating_add(OPACITY_STEP).min(100)
    } else {
        cur.saturating_sub(OPACITY_STEP).max(OPACITY_MIN)
    }
}

/// Phần trăm sang alpha 0..=255 (làm tròn).
pub fn opacity_alpha(percent: u8) -> u8 {
    ((percent.min(100) as u32 * 255 + 50) / 100) as u8
}

/// Có được tạo thêm ảnh ghim khi đang có `count`.
pub fn can_add(count: usize) -> bool {
    count < MAX_PINS
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
    fn display_size_rounds_nearest_and_min_one() {
        assert_eq!(display_size(640, 480, 100), (640, 480));
        assert_eq!(display_size(3, 3, 50), (2, 2));
        assert_eq!(display_size(5, 5, 10), (1, 1));
        assert_eq!(display_size(1, 1, 10), (1, 1));
    }

    #[test]
    fn zoom_steps_in_and_out() {
        assert_eq!(zoom_in(100, 100, 100), Some(110));
        assert_eq!(zoom_out(100), Some(90));
        assert_eq!(zoom_in(100, 100, 400), None);
        assert_eq!(zoom_out(10), None);
        assert_eq!(zoom_in(100, 100, 105), Some(110));
        assert_eq!(zoom_out(105), Some(100));
        assert_eq!(zoom_in(8000, 100, 200), None);
        assert_eq!(zoom_in(8000, 100, 175), Some(200));
    }

    #[test]
    fn zoom_about_center_and_corner() {
        assert_eq!(
            zoom_about(r(0, 0, 100, 100), p(50, 50), 100, 100, 200),
            r(-50, -50, 200, 200)
        );
        assert_eq!(
            zoom_about(r(0, 0, 100, 100), p(0, 0), 100, 100, 200),
            r(0, 0, 200, 200)
        );
    }

    #[test]
    fn zoom_about_negative_coordinates() {
        let out = zoom_about(r(-1920, 100, 100, 100), p(-1870, 150), 100, 100, 200);
        assert_eq!(out, r(-1970, 50, 200, 200));
    }

    #[test]
    fn zoom_about_large_values_do_not_overflow() {
        let out = zoom_about(
            r(i32::MAX - 10, i32::MIN + 10, 100, 100),
            p(i32::MIN, i32::MAX),
            100,
            100,
            400,
        );
        assert_eq!((out.w, out.h), (400, 400));
        let out = zoom_about(
            r(0, 0, 16384, 16384),
            p(i32::MAX, i32::MAX),
            4096,
            4096,
            400,
        );
        assert_eq!((out.w, out.h), (16384, 16384));
    }

    #[test]
    fn zoom_about_extreme_inputs_do_not_panic() {
        let out = zoom_about(
            r(i32::MAX, i32::MIN, i32::MAX, i32::MAX),
            p(i32::MIN, i32::MAX),
            u32::MAX,
            u32::MAX,
            400,
        );
        assert_eq!((out.w, out.h), (i32::MAX, i32::MAX));
    }

    #[test]
    fn fit_zoom_cases() {
        let area = r(0, 0, 1920, 1040);
        assert_eq!(fit_zoom(800, 600, area), 100);
        assert_eq!(fit_zoom(3840, 2160, area), 33);
        assert_eq!(fit_zoom(100000, 10, area), 10);
    }

    #[test]
    fn centered_matches_area_center() {
        assert_eq!(
            centered(100, 50, 100, r(0, 0, 1000, 500)),
            r(450, 225, 100, 50)
        );
        // Làm tròn xuống: (1920-101)/2 = 909.
        assert_eq!(
            centered(101, 50, 100, r(-1920, 0, 1920, 1080)),
            r(-1920 + 909, 515, 101, 50)
        );
    }

    /// Ảnh vừa vùng: như `centered`. Chiều nào lớn hơn vùng thì neo mép trên/trái vào vùng.
    #[test]
    fn placed_in_centers_or_anchors_start() {
        // Vừa: căn giữa như cũ.
        let area = r(0, 0, 1000, 500);
        assert_eq!(placed_in(100, 50, 100, area), centered(100, 50, 100, area));
        assert_eq!(placed_in(100, 50, 100, area), r(450, 225, 100, 50));
        // Vừa khít: như cũ.
        assert_eq!(placed_in(1000, 500, 100, area), r(0, 0, 1000, 500));
        // Cao hơn vùng ở 10%: mép trên = mép trên vùng, ngang căn giữa.
        let work = r(0, 40, 1920, 1040);
        assert_eq!(fit_zoom(1000, 100_000, work), 10);
        assert_eq!(placed_in(1000, 100_000, 10, work), r(910, 40, 100, 10_000));
        // Rộng hơn: mép trái = mép trái vùng, dọc căn giữa.
        assert_eq!(
            placed_in(100_000, 1000, 10, work),
            r(0, 40 + 470, 10_000, 100)
        );
        // Cả hai chiều lớn hơn: góc trên trái của vùng.
        assert_eq!(placed_in(30_000, 20_000, 10, work), r(0, 40, 3000, 2000));
        // Vùng tọa độ âm.
        let neg = r(-1920, -200, 1920, 1080);
        assert_eq!(
            placed_in(1000, 50_000, 10, neg),
            r(-1920 + 910, -200, 100, 5000)
        );
        assert_eq!(
            placed_in(50_000, 1000, 10, neg),
            r(-1920, -200 + 490, 5000, 100)
        );
        assert_eq!(placed_in(101, 50, 100, neg), centered(101, 50, 100, neg));
    }

    #[test]
    fn keep_visible_cases() {
        let m1 = r(0, 0, 1920, 1080);
        assert_eq!(
            keep_visible(r(100, 100, 400, 300), &[m1]),
            r(100, 100, 400, 300)
        );
        assert_eq!(
            keep_visible(r(3000, 100, 400, 300), &[m1]),
            r(1888, 100, 400, 300)
        );
        assert_eq!(
            keep_visible(r(-500, -400, 400, 300), &[m1]),
            r(-368, -268, 400, 300)
        );
        // Khung nhỏ hơn 32 px: cần cả khung.
        assert_eq!(
            keep_visible(r(2000, 100, 20, 20), &[m1]),
            r(1900, 100, 20, 20)
        );
        assert_eq!(keep_visible(r(5, 5, 20, 20), &[]), r(5, 5, 20, 20));
    }

    #[test]
    fn keep_visible_multi_monitor() {
        let left = r(-1920, 0, 1920, 1080);
        let main = r(0, 0, 1920, 1080);
        // Đã đủ 32 px trên màn hình trái.
        assert_eq!(
            keep_visible(r(-100, 100, 400, 300), &[left, main]),
            r(-100, 100, 400, 300)
        );
        // Giao nhiều nhất với main (x 500..900 nằm trong main, y ngoài hẳn mép dưới).
        assert_eq!(
            keep_visible(r(500, 2000, 400, 300), &[left, main]),
            r(500, 1048, 400, 300)
        );
        // Giao một phần màn hình trên cả hai trục nhưng chưa đủ 32 px: dời cả hai trục.
        assert_eq!(
            keep_visible(r(1900, 1070, 400, 300), &[left, main]),
            r(1888, 1048, 400, 300)
        );
        // Không giao màn hình nào: màn hình trái gần nhất.
        assert_eq!(
            keep_visible(r(-4000, 100, 400, 300), &[left, main]),
            r(-1920 + 32 - 400, 100, 400, 300)
        );
    }

    #[test]
    fn opacity_steps_and_alpha() {
        assert_eq!(opacity_step(100, true), 100);
        assert_eq!(opacity_step(100, false), 90);
        assert_eq!(opacity_step(20, false), 20);
        assert_eq!(opacity_step(25, false), 20);
        assert_eq!(opacity_alpha(100), 255);
        assert_eq!(opacity_alpha(20), 51);
        assert_eq!(opacity_alpha(60), 153);
    }

    #[test]
    fn zoom_allowed_limits_side_and_area() {
        // 4K 200% = 7680×4320 ≈ 33,2 triệu px².
        assert!(zoom_allowed(3840, 2160, 200));
        assert!(zoom_allowed(1920, 1080, 400));
        // 4K 250% = 9600×5400 ≈ 51,8 triệu px².
        assert!(!zoom_allowed(3840, 2160, 250));
        // Cạnh vừa MAX_SIDE nhưng diện tích quá lớn.
        assert!(!zoom_allowed(4096, 4096, 400));
        // ≤ 100% chỉ xét cạnh.
        assert!(zoom_allowed(8000, 6000, 100));
        assert!(!zoom_allowed(8000, 6000, 110));
        assert!(!zoom_allowed(20000, 10, 100));
        assert_eq!(zoom_in(3840, 2160, 200), None);
        assert_eq!(zoom_in(8000, 6000, 100), None);
        assert_eq!(zoom_in(3840, 2160, 175), Some(200));
    }

    #[test]
    fn can_add_limit() {
        assert!(can_add(31));
        assert!(!can_add(32));
    }
}
