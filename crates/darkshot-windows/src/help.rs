//! Cửa sổ Trợ giúp: không chặn, tối đa một cửa sổ. Thông điệp do vòng lặp của ứng dụng phân phối;
//! `pre_translate` lo bàn phím (Tab, Enter, Esc).

use crate::lang;
use crate::theme;
use crate::winutil;
use darkshot_core::i18n::{help_text, HelpKeys, Msg};
use darkshot_core::theme::{INPUT, TEXT};
use std::cell::Cell;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{DeleteObject, HDC, HFONT, HGDIOBJ};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, IsWindow, BS_DEFPUSHBUTTON, ES_AUTOVSCROLL, ES_MULTILINE,
    ES_READONLY, MSG, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC,
    WM_NCDESTROY, WS_BORDER, WS_GROUP, WS_TABSTOP, WS_VSCROLL,
};

const CLASS: PCWSTR = w!("darkshot-help");
/// Nút Đóng mang ID của IDOK để Enter trong hộp thoại bấm đúng nút này.
const ID_CLOSE: i32 = 1;
const ID_CANCEL: i32 = 2;
const ID_TEXT: i32 = 101;

thread_local! {
    /// HWND của cửa sổ đang mở (0 = không có).
    static OPEN: Cell<isize> = const { Cell::new(0) };
    static FONT: Cell<Option<HFONT>> = const { Cell::new(None) };
}

fn current() -> HWND {
    let h = OPEN.with(|o| o.get());
    let hwnd = HWND(h as *mut std::ffi::c_void);
    // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
    if h != 0 && unsafe { IsWindow(Some(hwnd)).as_bool() } {
        hwnd
    } else {
        HWND::default()
    }
}

/// Mở cửa sổ Trợ giúp với tổ hợp của các dòng phím tắt `keys`, hoặc đưa cửa sổ đang mở ra trước.
pub fn show(owner: HWND, keys: &HelpKeys) {
    let open = current();
    if !open.is_invalid() {
        winutil::bring_to_front(open);
        return;
    }
    let Some(hwnd) = winutil::create_popup(owner, CLASS, lang::tr(Msg::HelpTitle), wndproc) else {
        return;
    };
    // SAFETY: hwnd vừa tạo ở luồng này.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let font = winutil::system_font(dpi);
    FONT.with(|f| f.set(font));
    OPEN.with(|o| o.set(hwnd.0 as isize));

    // Ô EDIT cần \r\n để xuống dòng.
    let text = help_text(lang::current(), keys).replace('\n', "\r\n");
    let edit_style = WS_BORDER
        | WS_VSCROLL
        | WS_TABSTOP
        | WINDOW_STYLE((ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32);
    let edit = winutil::create_child(
        hwnd,
        font,
        dpi,
        w!("EDIT"),
        &text,
        edit_style,
        (12, 12, 496, 360),
        ID_TEXT,
    );
    let close = winutil::create_child(
        hwnd,
        font,
        dpi,
        w!("BUTTON"),
        lang::tr(Msg::BtnClose),
        WS_GROUP | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        (428, 382, 80, 28),
        ID_CLOSE,
    );
    theme::set_dark_control_theme(edit, theme::DarkClass::Explorer);
    theme::set_dark_control_theme(close, theme::DarkClass::Explorer);
    theme::apply_dark_title_bar(hwnd);
    winutil::finish_popup(hwnd, 520, 420, dpi);
    // Tiêu điểm ở nút Đóng để chữ không bị bôi chọn sẵn.
    // SAFETY: nút vừa tạo trong cửa sổ này.
    unsafe {
        let _ = SetFocus(Some(close));
    }
}

/// Đóng cửa sổ nếu đang mở; không có thì không làm gì.
pub fn close() {
    let h = current();
    if !h.is_invalid() {
        // SAFETY: h là cửa sổ Trợ giúp còn sống của luồng này; dọn dẹp ở WM_NCDESTROY.
        unsafe {
            let _ = DestroyWindow(h);
        }
    }
}

/// `true`: thông điệp đã được xử lý, vòng lặp bỏ qua.
pub fn pre_translate(msg: &MSG) -> bool {
    winutil::dialog_pre_translate(current(), msg, close)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, (wp.0 >> 16) & 0xFFFF);
            if code == 0 && (id == ID_CLOSE || id == ID_CANCEL) {
                close();
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            close();
            LRESULT(0)
        }
        // Ô chữ chỉ đọc gửi WM_CTLCOLORSTATIC, ô sửa được gửi WM_CTLCOLOREDIT: cùng chữ TEXT, nền INPUT.
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
            theme::ctl_color(HDC(wp.0 as *mut std::ffi::c_void), TEXT, INPUT)
        }
        WM_NCDESTROY => {
            // Cửa sổ đã mất hết con: giải phóng phông và quên handle, dù đóng bằng đường nào.
            OPEN.with(|o| o.set(0));
            if let Some(f) = FONT.with(|f| f.take()) {
                // SAFETY: phông do CreateFontIndirectW tạo, không còn điều khiển nào dùng.
                unsafe {
                    let _ = DeleteObject(HGDIOBJ(f.0));
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, RegisterClassW,
        WINDOW_EX_STYLE, WNDCLASSW, WS_CAPTION,
    };

    unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: chuyển nguyên tham số.
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    fn gdi_count() -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    }

    /// Mở, mở lần hai, đóng; ba vòng, số đối tượng GDI không tăng dần.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored help_window`.
    #[test]
    #[ignore]
    fn help_window_opens_and_closes() {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, hủy ở cuối.
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot-help-test-parent");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc_),
                hInstance: hinst.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!("parent"),
                WS_CAPTION,
                0,
                0,
                10,
                10,
                None,
                None,
                Some(hinst.into()),
                None,
            )
            .unwrap()
        };
        let find = || unsafe { FindWindowW(w!("darkshot-help"), None) };
        let mut counts = Vec::new();
        let keys = HelpKeys {
            region: Some("PrintScreen".into()),
            save_full: None,
            upload_full: Some("Ctrl+PrintScreen".into()),
        };
        for _ in 0..3 {
            show(parent, &keys);
            let first = find().expect("window not created");
            show(parent, &keys);
            assert_eq!(
                find().expect("window vanished"),
                first,
                "must stay a single window"
            );
            close();
            assert!(find().is_err(), "window must be gone after close");
            counts.push(gdi_count());
        }
        close(); // không có cửa sổ: không làm gì
        assert!(
            counts[2] <= counts[0],
            "GDI objects grew across opens: {counts:?}"
        );
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
    }
}
