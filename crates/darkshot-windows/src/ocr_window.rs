//! Cửa sổ "Chữ nhận dạng" (chưa làm: chỉ khung cho kiểm thử).
#![allow(dead_code)]

use crate::ocr::{OcrError, OcrResult};
use darkshot_core::geometry::Rect;
use darkshot_core::i18n::Lang;
use darkshot_core::image::Image;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MSG, WM_APP};

/// Kết quả nhận dạng gửi về cửa sổ: WPARAM = token, LPARAM = `Box` của `ocr::start`.
pub const WM_OCR_RESULT: u32 = WM_APP + 12;
const MIN_W: i32 = 0;
const MIN_H: i32 = 0;
const DEFAULT_W: i32 = 0;
const DEFAULT_H: i32 = 0;
const MAX_WINDOWS: usize = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    field: Rect,
    edit: Rect,
    status: Rect,
    copy: Rect,
    close: Rect,
}

fn button_width(_text_px: i32, _dpi: u32) -> i32 {
    0
}

fn layout(_cw: i32, _ch: i32, _dpi: u32, _copy_text: i32, _close_text: i32) -> Layout {
    let z = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    Layout {
        field: z,
        edit: z,
        status: z,
        copy: z,
        close: z,
    }
}

fn token_matches(_expected: Option<usize>, _got: usize) -> bool {
    true
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct View {
    text: String,
    status: String,
    editable: bool,
    can_copy: bool,
    muted: bool,
}

fn busy_view(_lang: Lang) -> View {
    View::default()
}

fn view_of(_lang: Lang, _res: &Result<OcrResult, OcrError>) -> View {
    View::default()
}

fn lang_label(_tag: &str, _name: &str) -> String {
    String::new()
}

fn to_crlf(s: &str) -> String {
    s.to_string()
}

type Starter = fn(Image, HWND, u32, usize) -> Result<(), String>;

pub fn open(image: Image) -> Result<(), String> {
    open_with(image, crate::ocr::start)
}

fn open_with(_image: Image, _start: Starter) -> Result<(), String> {
    Err("not implemented".into())
}

pub fn close_all() {}

pub fn pre_translate(_msg: &MSG) -> bool {
    false
}

pub fn set_notifier(_f: fn(darkshot_core::actions::NoticeKind, &str, &str)) {}

#[cfg(test)]
fn test_windows() -> Vec<(HWND, usize)> {
    Vec::new()
}

#[cfg(test)]
fn busy(_hwnd: HWND) -> bool {
    false
}

#[cfg(test)]
fn edit_of(_hwnd: HWND) -> HWND {
    HWND::default()
}

#[cfg(test)]
thread_local! {
    static FREED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winutil::scale;
    use darkshot_core::i18n::{ocr_status, t, Msg};
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetWindowTextLengthW, GetWindowTextW, IsWindow, PeekMessageW,
        PostMessageW, TranslateMessage, PM_NOREMOVE, PM_REMOVE,
    };

    const LANGS: [Lang; 2] = [Lang::Vi, Lang::En];
    const DPIS: [u32; 5] = [96, 120, 144, 168, 192];

    /// Bề rộng chữ bi quan (mọi ký tự rộng 10 DIP, phông đậm) để kiểm thử không cắt nút.
    fn text_px(s: &str, dpi: u32) -> i32 {
        s.chars().count() as i32 * scale(10, dpi)
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
    }

    fn within(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.right() <= outer.right()
            && inner.bottom() <= outer.bottom()
    }

    #[test]
    fn sizes_match_design() {
        assert_eq!((MIN_W, MIN_H), (420, 260));
        assert_eq!((DEFAULT_W, DEFAULT_H), (560, 380));
        assert_eq!(MAX_WINDOWS, 8);
    }

    #[test]
    fn button_width_fits_label() {
        for dpi in DPIS {
            assert!(button_width(0, dpi) >= scale(80, dpi));
            for px in [0, 30, 80, 200] {
                let w = button_width(px, dpi);
                assert!(w >= px + 2 * scale(12, dpi), "dpi {dpi} px {px}: {w}");
            }
        }
    }

    /// Ở cỡ tối thiểu và mặc định, mọi DPI 96–192 và hai ngôn ngữ: ô chữ, dòng trạng thái, hai nút
    /// nằm trong vùng client, không chồng nhau, nút đủ rộng cho chữ; ô chữ nằm trong khung của nó.
    #[test]
    fn layout_fits_without_overlap() {
        for lang in LANGS {
            for dpi in DPIS {
                let copy = text_px(t(lang, Msg::OcrCopy), dpi);
                let close = text_px(t(lang, Msg::OcrClose), dpi);
                for (w, h) in [(MIN_W, MIN_H), (DEFAULT_W, DEFAULT_H), (1200, 900)] {
                    let (cw, ch) = (scale(w, dpi), scale(h, dpi));
                    let client = Rect {
                        x: 0,
                        y: 0,
                        w: cw,
                        h: ch,
                    };
                    let l = layout(cw, ch, dpi, copy, close);
                    let ctx = format!("{lang:?} dpi {dpi} {w}x{h}: {l:?}");
                    let parts = [l.field, l.status, l.copy, l.close];
                    for (i, a) in parts.iter().enumerate() {
                        assert!(a.w > 0 && a.h > 0, "{ctx}");
                        assert!(within(*a, client), "{ctx}");
                        for b in &parts[i + 1..] {
                            assert!(!overlaps(*a, *b), "{ctx}");
                        }
                    }
                    assert!(within(l.edit, l.field) && l.edit != l.field, "{ctx}");
                    assert!(l.edit.h >= scale(100, dpi), "{ctx}");
                    assert!(l.copy.w >= copy + 2 * scale(12, dpi), "{ctx}");
                    assert!(l.close.w >= close + 2 * scale(12, dpi), "{ctx}");
                    // Chép bên trái Đóng, cùng hàng, sát mép phải; trạng thái ở trên hàng nút.
                    assert!(l.copy.right() <= l.close.x, "{ctx}");
                    assert_eq!(l.copy.y, l.close.y, "{ctx}");
                    assert_eq!(l.close.right(), cw - scale(12, dpi), "{ctx}");
                    assert!(l.status.bottom() <= l.copy.y, "{ctx}");
                    assert!(l.field.bottom() <= l.status.y, "{ctx}");
                    assert!(l.status.h >= scale(16, dpi), "{ctx}");
                }
            }
        }
    }

    /// Chỉ nhận kết quả mang đúng token của cửa sổ, và chỉ một lần (sau đó token là `None`).
    #[test]
    fn token_mismatch_rejected() {
        assert!(token_matches(Some(5), 5));
        assert!(!token_matches(Some(5), 4));
        assert!(!token_matches(Some(5), 6));
        assert!(!token_matches(None, 5));
        assert!(!token_matches(None, 0));
    }

    #[test]
    fn lang_label_has_name_and_tag() {
        assert_eq!(
            lang_label("en-US", "English (United States)"),
            "English (United States), en-US"
        );
        assert_eq!(lang_label("vi", ""), "vi");
        assert_eq!(lang_label("", "Tiếng Việt"), "Tiếng Việt");
    }

    #[test]
    fn crlf_normalized() {
        assert_eq!(to_crlf("a\nb\r\nc\rd"), "a\r\nb\r\nc\r\nd");
        assert_eq!(to_crlf(""), "");
        assert_eq!(to_crlf("x"), "x");
    }

    #[test]
    fn views_for_each_result() {
        for lang in LANGS {
            let busy = busy_view(lang);
            assert_eq!(busy.text, t(lang, Msg::OcrBusy));
            assert!(!busy.editable && !busy.can_copy && busy.muted);
            assert_eq!(busy.status, "");

            let ok = Ok(OcrResult {
                text: "Hello\r\nworld".into(),
                lang_tag: "en-US".into(),
                lang_name: "English".into(),
                lines: 2,
            });
            let v = view_of(lang, &ok);
            assert_eq!(v.text, "Hello\r\nworld");
            assert!(v.editable && v.can_copy && !v.muted);
            assert_eq!(v.status, ocr_status(lang, "English, en-US", 2));

            let empty = Ok(OcrResult {
                text: String::new(),
                lang_tag: "en-US".into(),
                lang_name: "English".into(),
                lines: 0,
            });
            let v = view_of(lang, &empty);
            assert_eq!(v.text, t(lang, Msg::OcrEmpty));
            assert!(!v.editable && !v.can_copy);
            assert_eq!(v.status, ocr_status(lang, "English, en-US", 0));

            let v = view_of(lang, &Err(OcrError::NoLanguage));
            assert_eq!(v.text, t(lang, Msg::OcrNoLanguage));
            assert!(!v.editable && !v.can_copy);
            assert_eq!(v.status, "");

            let v = view_of(
                lang,
                &Err(OcrError::Failed("RecognizeAsync: 0x80004005".into())),
            );
            assert_eq!(v.text, t(lang, Msg::OcrFailed));
            assert!(!v.text.contains("0x8") && !v.status.contains("0x8"));
            assert!(!v.editable && !v.can_copy);
        }
    }

    // ---- Chạy tay (mở cửa sổ thật, nhỏ, không chặn):
    // cargo test -p darkshot-windows ocr_window::tests::manual -- --ignored --test-threads=1

    fn pump(ms: u64, done: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_millis(ms);
        let mut m = MSG::default();
        loop {
            // SAFETY: vòng lặp thông điệp của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
                    if !pre_translate(&m) {
                        let _ = TranslateMessage(&m);
                        DispatchMessageW(&m);
                    }
                }
            }
            if done() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    fn text_of(h: HWND) -> String {
        // SAFETY: đọc chữ của điều khiển thuộc luồng này vào bộ đệm cục bộ.
        unsafe {
            let n = GetWindowTextLengthW(h).max(0) as usize;
            let mut buf = vec![0u16; n + 1];
            let got = GetWindowTextW(h, &mut buf).max(0) as usize;
            String::from_utf16_lossy(&buf[..got])
        }
    }

    fn post_fake(h: HWND, token: usize, text: &str) {
        let res: Result<OcrResult, OcrError> = Ok(OcrResult {
            text: text.into(),
            lang_tag: "en-US".into(),
            lang_name: "English".into(),
            lines: 1,
        });
        let p = Box::into_raw(Box::new(res));
        // SAFETY: gửi tới cửa sổ của luồng này; gửi lỗi thì lấy lại Box.
        unsafe {
            if PostMessageW(Some(h), WM_OCR_RESULT, WPARAM(token), LPARAM(p as isize)).is_err() {
                drop(Box::from_raw(p));
                panic!("PostMessageW failed");
            }
        }
    }

    fn no_ocr(_: Image, _: HWND, _: u32, _: usize) -> Result<(), String> {
        Ok(())
    }

    fn tiny() -> Image {
        Image {
            width: 8,
            height: 8,
            bgra: vec![255; 8 * 8 * 4],
        }
    }

    fn gui_counts() -> (u32, u32) {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe {
            (
                GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS),
                GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS),
            )
        }
    }

    fn lang_now() -> Lang {
        crate::lang::current()
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_recognizes_hello() {
        close_all();
        let img = crate::ocr::tests::text_image(600, 120, Some("Hello darkshot 123"));
        open(img).expect("open");
        let wins = test_windows();
        assert_eq!(wins.len(), 1);
        let h = wins[0].0;
        let edit = edit_of(h);
        assert!(!edit.is_invalid());
        assert!(busy(h), "must start busy");
        assert!(pump(30_000, || !busy(h)), "no result within 30 s");
        let text = text_of(edit);
        if text == t(lang_now(), Msg::OcrNoLanguage) {
            println!("NoLanguage: may khong co goi OCR");
        } else {
            assert!(text.contains("Hello"), "{text:?}");
        }
        close_all();
        assert!(test_windows().is_empty());
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_open_close_no_leak() {
        close_all();
        for _ in 0..2 {
            open_with(tiny(), no_ocr).expect("open");
            close_all();
        }
        pump(200, || false);
        let before = gui_counts();
        for _ in 0..20 {
            open(tiny()).expect("open");
            pump(20, || false);
            close_all();
            assert!(test_windows().is_empty());
        }
        // Kết quả của 20 lượt nhận dạng tới cửa sổ đã đóng: bị bỏ, không cửa sổ nào còn.
        pump(5_000, || false);
        let after = gui_counts();
        assert!(
            after.0 <= before.0 && after.1 <= before.1,
            "GDI/USER grew: {before:?} -> {after:?}"
        );
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_close_while_recognizing() {
        close_all();
        let img = crate::ocr::tests::text_image(1920, 1080, Some("Hello darkshot 123"));
        open(img).expect("open");
        let (h, _) = test_windows()[0];
        close_all();
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        pump(8_000, || false);
        let mut m = MSG::default();
        // SAFETY: chỉ nhìn hàng đợi của luồng này.
        let left = unsafe { PeekMessageW(&mut m, None, WM_OCR_RESULT, WM_OCR_RESULT, PM_NOREMOVE) };
        assert!(!left.as_bool(), "a result is left in the queue");
        assert!(test_windows().is_empty());
    }

    /// Kết quả token cũ bị bỏ (và giải phóng); kết quả đúng được dùng đúng một lần; kết quả tới sau
    /// khi đóng (còn trong hàng đợi) được vét và giải phóng khi hủy cửa sổ.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_late_and_stale_results_freed() {
        close_all();
        FREED.with(|f| f.set(0));
        open_with(tiny(), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        let edit = edit_of(h);
        post_fake(h, token.wrapping_add(1000), "stale");
        pump(300, || false);
        assert_eq!(FREED.with(|f| f.get()), 1);
        assert!(busy(h));
        assert_eq!(text_of(edit), t(lang_now(), Msg::OcrBusy));

        post_fake(h, token, "Hello");
        assert!(pump(2_000, || !busy(h)));
        assert_eq!(FREED.with(|f| f.get()), 2);
        assert_eq!(text_of(edit), "Hello");

        post_fake(h, token, "again");
        pump(300, || false);
        assert_eq!(FREED.with(|f| f.get()), 3);
        assert_eq!(text_of(edit), "Hello");

        post_fake(h, token, "late 1");
        post_fake(h, token.wrapping_add(7), "late 2");
        close_all();
        assert_eq!(FREED.with(|f| f.get()), 5);
        assert!(test_windows().is_empty());
    }

    /// Quá 8 cửa sổ: cái cũ nhất đóng.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_at_most_eight_windows() {
        close_all();
        open_with(tiny(), no_ocr).expect("open");
        let first = test_windows()[0].0;
        for _ in 0..8 {
            open_with(tiny(), no_ocr).expect("open");
        }
        let wins = test_windows();
        assert_eq!(wins.len(), 8);
        assert!(wins.iter().all(|(h, _)| *h != first));
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(!unsafe { IsWindow(Some(first)) }.as_bool());
        close_all();
        assert!(test_windows().is_empty());
    }
}
