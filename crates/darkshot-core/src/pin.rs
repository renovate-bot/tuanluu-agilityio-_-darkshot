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

/// Cỡ hiển thị của ảnh w×h ở mức `zoom` (%), làm tròn gần nhất, mỗi chiều ≥ 1.
pub fn display_size(_w: u32, _h: u32, _zoom: u16) -> (i32, i32) {
    todo!()
}

/// Bậc kế tiếp lớn hơn trong ZOOM_STEPS mà cỡ hiển thị mỗi chiều ≤ MAX_SIDE.
pub fn zoom_in(_w: u32, _h: u32, _zoom: u16) -> Option<u16> {
    todo!()
}

/// Bậc kế tiếp nhỏ hơn trong ZOOM_STEPS.
pub fn zoom_out(_zoom: u16) -> Option<u16> {
    todo!()
}

/// Khung ảnh mới khi đổi mức, giữ `anchor` đứng yên trên cùng điểm ảnh.
pub fn zoom_about(
    _image_rect: Rect,
    _anchor: Point,
    _img_w: u32,
    _img_h: u32,
    _new_zoom: u16,
) -> Rect {
    todo!()
}

/// Bậc lớn nhất ≤ 100 để ảnh vừa FIT_PERCENT% của `area` mỗi chiều.
pub fn fit_zoom(_img_w: u32, _img_h: u32, _area: Rect) -> u16 {
    todo!()
}

/// Khung ảnh căn giữa `area` ở mức `zoom`.
pub fn centered(_img_w: u32, _img_h: u32, _zoom: u16, _area: Rect) -> Rect {
    todo!()
}

/// Dời `r` ít nhất có thể sao cho đủ MIN_VISIBLE px mỗi chiều nằm trong một màn hình.
pub fn keep_visible(_r: Rect, _monitors: &[Rect]) -> Rect {
    todo!()
}

/// Độ mờ kế tiếp (phần trăm); `up` là đục hơn. Kẹp [OPACITY_MIN, 100].
pub fn opacity_step(_cur: u8, _up: bool) -> u8 {
    todo!()
}

/// Phần trăm sang alpha 0..=255 (làm tròn).
pub fn opacity_alpha(_percent: u8) -> u8 {
    todo!()
}

/// Có được tạo thêm ảnh ghim khi đang có `count`.
pub fn can_add(_count: usize) -> bool {
    todo!()
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
        // Khung giao hai màn hình nhưng chỉ main đủ lớn hơn: giữ nguyên vì main đã đủ.
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
    fn can_add_limit() {
        assert!(can_add(31));
        assert!(!can_add(32));
    }
}
