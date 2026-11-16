//! Chụp toàn bộ màn hình ảo vào bộ nhớ.

use shotme_core::geometry::{Point, Rect};
use shotme_core::image::Image;

pub struct Screenshot {
    /// Toàn bộ màn hình ảo, pixel vật lý.
    pub image: Image,
    /// Góc trên trái của màn hình ảo trong tọa độ màn hình (có thể âm).
    pub origin: Point,
    /// Từng màn hình, đã đổi sang tọa độ ảnh (trừ origin).
    pub monitors: Vec<Rect>,
}

pub fn capture_virtual_screen() -> windows::core::Result<Screenshot> {
    todo!()
}

pub fn to_image_coords(screen: Rect, origin: Point) -> Rect {
    let _ = (screen, origin);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_coords_with_negative_origin() {
        let r = to_image_coords(
            Rect {
                x: -1920,
                y: -200,
                w: 1920,
                h: 1080,
            },
            Point { x: -1920, y: -200 },
        );
        assert_eq!(
            r,
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080
            }
        );
        let r = to_image_coords(
            Rect {
                x: 0,
                y: 0,
                w: 2560,
                h: 1440,
            },
            Point { x: -1920, y: -200 },
        );
        assert_eq!(
            r,
            Rect {
                x: 1920,
                y: 200,
                w: 2560,
                h: 1440
            }
        );
    }

    #[test]
    fn capture_matches_virtual_screen() {
        let s = capture_virtual_screen().unwrap();
        assert!(s.image.width > 0 && s.image.height > 0);
        assert_eq!(
            s.image.bgra.len(),
            (s.image.width * s.image.height * 4) as usize
        );
        assert!(!s.monitors.is_empty());
        // mọi màn hình nằm trọn trong ảnh
        assert!(s.monitors.iter().all(|m| m.x >= 0
            && m.y >= 0
            && m.right() <= s.image.width as i32
            && m.bottom() <= s.image.height as i32));
    }
}
