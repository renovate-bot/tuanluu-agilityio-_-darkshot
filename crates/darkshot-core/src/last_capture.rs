//! Lần chụp vùng trước: vùng (tọa độ màn hình) và hành động đã chọn, để "Chụp lại vùng trước".

use crate::actions::Action;
use crate::geometry::{Point, Rect, MIN_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LastCapture {
    /// Tọa độ MÀN HÌNH.
    pub rect: Rect,
    pub action: Action,
}

pub const FILE_VERSION: u32 = 1;

fn action_code(a: Action) -> Option<&'static str> {
    match a {
        Action::Save => Some("save"),
        Action::Copy => Some("copy"),
        Action::Upload => Some("upload"),
        Action::Pin => Some("pin"),
        Action::Cancel => None,
    }
}

fn action_from_code(s: &str) -> Option<Action> {
    match s {
        "save" => Some(Action::Save),
        "copy" => Some(Action::Copy),
        "upload" => Some(Action::Upload),
        "pin" => Some(Action::Pin),
        _ => None,
    }
}

pub fn to_json(l: &LastCapture) -> String {
    todo!()
}

/// None khi JSON hỏng, version khác 1, w/h < MIN_SIZE, action lạ hoặc "cancel".
pub fn from_json(s: &str) -> Option<LastCapture> {
    todo!()
}

/// Lần chụp trước sau một lần chụp vùng kết thúc bằng `action` với vùng `rect` (tọa độ màn hình):
/// Cancel giữ `prev`.
pub fn remember(prev: Option<LastCapture>, action: Action, rect: Rect) -> Option<LastCapture> {
    todo!()
}

/// Vùng cắt (tọa độ ẢNH) để lặp lại; None khi vùng không giao màn hình nào hoặc phần còn trong
/// ảnh < MIN_SIZE. `monitors_screen` ở tọa độ màn hình.
pub fn repeat_rect(
    last: &LastCapture,
    origin: Point,
    img_w: i32,
    img_h: i32,
    monitors_screen: &[Rect],
) -> Option<Rect> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rc(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    fn lc(rect: Rect, action: Action) -> LastCapture {
        LastCapture { rect, action }
    }
    fn p(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    #[test]
    fn json_round_trip_all_actions() {
        for a in [Action::Save, Action::Copy, Action::Upload, Action::Pin] {
            let l = lc(rc(10, 20, 300, 200), a);
            assert_eq!(from_json(&to_json(&l)), Some(l));
        }
    }

    #[test]
    fn json_round_trip_negative_coords() {
        let l = lc(rc(-1500, -40, 300, 200), Action::Copy);
        assert_eq!(from_json(&to_json(&l)), Some(l));
    }

    #[test]
    fn from_json_rejects_bad_input() {
        let ok = r#"{"version":1,"x":1,"y":2,"w":30,"h":40,"action":"save"}"#;
        assert!(from_json(ok).is_some());
        for bad in [
            "",
            "rác",
            "{",
            "[]",
            "null",
            r#"{"version":2,"x":1,"y":2,"w":30,"h":40,"action":"save"}"#,
            r#"{"version":1,"x":1,"y":2,"w":2,"h":40,"action":"save"}"#,
            r#"{"version":1,"x":1,"y":2,"w":30,"h":2,"action":"save"}"#,
            r#"{"version":1,"x":1,"y":2,"w":30,"h":40,"action":"cancel"}"#,
            r#"{"version":1,"x":1,"y":2,"w":30,"h":40,"action":"lạ"}"#,
            r#"{"version":1,"x":1,"y":2,"w":30,"h":40}"#,
            r#"{"version":1,"x":1,"y":2,"w":30,"action":"save"}"#,
            r#"{"x":1,"y":2,"w":30,"h":40,"action":"save"}"#,
            r#"{"version":1,"x":99999999999,"y":2,"w":30,"h":40,"action":"save"}"#,
            r#"{"version":1,"x":"1","y":2,"w":30,"h":40,"action":"save"}"#,
        ] {
            assert_eq!(from_json(bad), None, "{bad}");
        }
    }

    #[test]
    fn remember_cancel_keeps_prev() {
        let r = rc(0, 0, 50, 50);
        assert_eq!(remember(None, Action::Cancel, r), None);
        let prev = lc(rc(5, 5, 10, 10), Action::Pin);
        assert_eq!(remember(Some(prev), Action::Cancel, r), Some(prev));
    }

    #[test]
    fn remember_replaces_prev() {
        let r = rc(0, 0, 50, 50);
        let prev = lc(rc(5, 5, 10, 10), Action::Pin);
        assert_eq!(
            remember(Some(prev), Action::Save, r),
            Some(lc(r, Action::Save))
        );
        assert_eq!(remember(None, Action::Upload, r), Some(lc(r, Action::Upload)));
    }

    #[test]
    fn repeat_rect_main_monitor() {
        let mons = [rc(0, 0, 1920, 1080)];
        let l = lc(rc(100, 100, 300, 200), Action::Save);
        assert_eq!(
            repeat_rect(&l, p(0, 0), 1920, 1080, &mons),
            Some(rc(100, 100, 300, 200))
        );
    }

    #[test]
    fn repeat_rect_negative_origin_subtracts_once() {
        let mons = [rc(-1920, 0, 1920, 1080), rc(0, 0, 1920, 1080)];
        let l = lc(rc(-1000, 100, 300, 200), Action::Save);
        assert_eq!(
            repeat_rect(&l, p(-1920, 0), 3840, 1080, &mons),
            Some(rc(920, 100, 300, 200))
        );
    }

    #[test]
    fn repeat_rect_removed_monitor_is_none() {
        // màn hình trái đã rút: ảnh chỉ còn màn hình chính
        let mons = [rc(0, 0, 1920, 1080)];
        let l = lc(rc(-1000, 100, 300, 200), Action::Save);
        assert_eq!(repeat_rect(&l, p(0, 0), 1920, 1080, &mons), None);
    }

    #[test]
    fn repeat_rect_clips_right_edge() {
        let mons = [rc(0, 0, 1920, 1080)];
        let l = lc(rc(1800, 100, 300, 200), Action::Save);
        assert_eq!(
            repeat_rect(&l, p(0, 0), 1920, 1080, &mons),
            Some(rc(1800, 100, 120, 200))
        );
        // còn đúng MIN_SIZE thì giữ
        let l = lc(rc(1917, 100, 300, 200), Action::Save);
        assert_eq!(
            repeat_rect(&l, p(0, 0), 1920, 1080, &mons),
            Some(rc(1917, 100, 3, 200))
        );
        // còn < MIN_SIZE thì bỏ
        let l = lc(rc(1918, 100, 300, 200), Action::Save);
        assert_eq!(repeat_rect(&l, p(0, 0), 1920, 1080, &mons), None);
    }

    #[test]
    fn repeat_rect_extreme_values_do_not_panic() {
        let mons = [rc(0, 0, 1920, 1080)];
        let l = lc(rc(i32::MIN, i32::MAX, i32::MAX, i32::MAX), Action::Save);
        assert_eq!(
            repeat_rect(&l, p(i32::MAX, i32::MIN), 1920, 1080, &mons),
            None
        );
        let l = lc(rc(i32::MAX, i32::MAX, i32::MAX, i32::MAX), Action::Save);
        assert_eq!(
            repeat_rect(&l, p(i32::MIN, i32::MIN), i32::MAX, i32::MAX, &mons),
            None
        );
    }
}
