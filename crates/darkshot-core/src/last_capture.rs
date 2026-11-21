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
        Action::Cancel | Action::Ocr | Action::AddStep | Action::Scroll | Action::RecordGif => None,
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
    serde_json::json!({
        "version": FILE_VERSION,
        "x": l.rect.x,
        "y": l.rect.y,
        "w": l.rect.w,
        "h": l.rect.h,
        "action": action_code(l.action).unwrap_or("save"),
    })
    .to_string()
}

/// None khi JSON hỏng, version khác 1, w/h < MIN_SIZE, action lạ hoặc "cancel".
pub fn from_json(s: &str) -> Option<LastCapture> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    if v.get("version")?.as_u64()? != u64::from(FILE_VERSION) {
        return None;
    }
    let num = |k: &str| -> Option<i32> { i32::try_from(v.get(k)?.as_i64()?).ok() };
    let rect = Rect {
        x: num("x")?,
        y: num("y")?,
        w: num("w")?,
        h: num("h")?,
    };
    if rect.w < MIN_SIZE || rect.h < MIN_SIZE {
        return None;
    }
    let action = action_from_code(v.get("action")?.as_str()?)?;
    Some(LastCapture { rect, action })
}

/// Lần chụp trước sau một lần chụp vùng kết thúc bằng `action` với vùng `rect` (tọa độ màn hình):
/// Cancel, Ocr, AddStep và Scroll giữ `prev` (không phải lần chụp để lặp lại).
pub fn remember(prev: Option<LastCapture>, action: Action, rect: Rect) -> Option<LastCapture> {
    if matches!(
        action,
        Action::Cancel | Action::Ocr | Action::AddStep | Action::Scroll
    ) {
        return prev;
    }
    Some(LastCapture { rect, action })
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
    let (ox, oy) = (i64::from(origin.x), i64::from(origin.y));
    let r = last.rect;
    // Tọa độ ảnh, i64 để không tràn.
    let (x0, y0) = (i64::from(r.x) - ox, i64::from(r.y) - oy);
    let (x1, y1) = (x0 + i64::from(r.w), y0 + i64::from(r.h));
    let (cx0, cy0) = (x0.max(0), y0.max(0));
    let (cx1, cy1) = (x1.min(i64::from(img_w)), y1.min(i64::from(img_h)));
    if cx1 - cx0 < i64::from(MIN_SIZE) || cy1 - cy0 < i64::from(MIN_SIZE) {
        return None;
    }
    let touches = monitors_screen.iter().any(|m| {
        let mx0 = i64::from(m.x) - ox;
        let my0 = i64::from(m.y) - oy;
        let (mx1, my1) = (mx0 + i64::from(m.w), my0 + i64::from(m.h));
        cx0.max(mx0) < cx1.min(mx1) && cy0.max(my0) < cy1.min(my1)
    });
    if !touches {
        return None;
    }
    Some(Rect {
        x: i32::try_from(cx0).ok()?,
        y: i32::try_from(cy0).ok()?,
        w: i32::try_from(cx1 - cx0).ok()?,
        h: i32::try_from(cy1 - cy0).ok()?,
    })
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
    fn remember_ocr_keeps_prev() {
        let r = rc(1, 2, 30, 40);
        let prev = lc(rc(5, 5, 10, 10), Action::Pin);
        assert_eq!(remember(None, Action::Ocr, r), None);
        assert_eq!(remember(Some(prev), Action::Ocr, r), Some(prev));
    }

    #[test]
    fn remember_add_step_keeps_prev() {
        let r = rc(1, 2, 30, 40);
        let prev = lc(rc(5, 5, 10, 10), Action::Pin);
        assert_eq!(remember(None, Action::AddStep, r), None);
        assert_eq!(remember(Some(prev), Action::AddStep, r), Some(prev));
    }

    #[test]
    fn remember_scroll_keeps_prev() {
        let r = rc(1, 2, 30, 40);
        let prev = lc(rc(5, 5, 10, 10), Action::Pin);
        assert_eq!(remember(None, Action::Scroll, r), None);
        assert_eq!(remember(Some(prev), Action::Scroll, r), Some(prev));
    }

    #[test]
    fn remember_record_gif_keeps_prev() {
        let r = rc(1, 2, 30, 40);
        let prev = lc(rc(5, 5, 10, 10), Action::Pin);
        assert_eq!(remember(None, Action::RecordGif, r), None);
        assert_eq!(remember(Some(prev), Action::RecordGif, r), Some(prev));
    }

    #[test]
    fn json_never_writes_record() {
        let s = to_json(&lc(rc(1, 2, 30, 40), Action::RecordGif));
        assert!(!s.contains("record"), "{s}");
        assert!(!s.contains("gif"), "{s}");
        assert_eq!(from_json(&s.replace("save", "record")), None);
        assert_eq!(from_json(&s.replace("save", "record_gif")), None);
        assert_eq!(from_json(&s.replace("save", "gif")), None);
    }

    #[test]
    fn json_never_writes_scroll() {
        let s = to_json(&lc(rc(1, 2, 30, 40), Action::Scroll));
        assert!(!s.contains("scroll"), "{s}");
        assert_eq!(from_json(&s.replace("save", "scroll")), None);
    }

    #[test]
    fn remember_replaces_prev() {
        let r = rc(0, 0, 50, 50);
        let prev = lc(rc(5, 5, 10, 10), Action::Pin);
        assert_eq!(
            remember(Some(prev), Action::Save, r),
            Some(lc(r, Action::Save))
        );
        assert_eq!(
            remember(None, Action::Upload, r),
            Some(lc(r, Action::Upload))
        );
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
