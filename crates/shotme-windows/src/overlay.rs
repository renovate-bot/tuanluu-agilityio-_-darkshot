//! Lớp phủ toàn màn hình để chọn vùng.

use crate::capture::Screenshot;
use shotme_core::actions::Action;
use shotme_core::geometry::{Point, Rect};

/// Hiện lớp phủ trên ảnh `shot` và chờ người dùng chọn. `Rect` trong tọa độ ảnh; `None` khi hủy.
pub fn pick_region(shot: &Screenshot) -> Option<(Action, Rect)> {
    let _ = shot;
    todo!()
}

/// Nút của thanh công cụ dưới điểm `p`; bốn nút vuông xếp ngang theo thứ tự tải lên, lưu, chép, hủy.
pub fn button_at(toolbar: Rect, p: Point) -> Option<Action> {
    let _ = (toolbar, p);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_button_hit_testing() {
        let bar = Rect {
            x: 100,
            y: 100,
            w: 144,
            h: 36,
        };
        let at = |x, y| button_at(bar, Point { x, y });
        assert_eq!(at(110, 110), Some(Action::Upload));
        assert_eq!(at(150, 110), Some(Action::Save));
        assert_eq!(at(190, 110), Some(Action::Copy));
        assert_eq!(at(230, 110), Some(Action::Cancel));
        assert_eq!(at(300, 110), None);
    }
}
