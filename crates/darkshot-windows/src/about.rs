//! Cửa sổ Thông tin: không chặn, tối đa một cửa sổ. Thông điệp do vòng lặp của ứng dụng phân phối;
//! `pre_translate` lo bàn phím (Tab, Enter, Esc).

use crate::lang;
use crate::winutil;
use darkshot_core::i18n::{about_version, Msg};
use std::cell::Cell;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetSysColorBrush, SetBkMode, COLOR_BTNFACE, HDC, HFONT, HGDIOBJ, TRANSPARENT,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, IsWindow, SendMessageW, BS_DEFPUSHBUTTON, MSG, STM_SETICON,
    WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CTLCOLORSTATIC, WM_NCDESTROY, WS_GROUP, WS_TABSTOP,
};

const CLASS: PCWSTR = w!("darkshot-about");
/// Nút Đóng mang ID của IDOK để Enter trong hộp thoại bấm đúng nút này.
const ID_CLOSE: i32 = 1;
const ID_CANCEL: i32 = 2;
/// Kiểu của STATIC: biểu tượng, và chữ căn giữa.
const SS_ICON: WINDOW_STYLE = WINDOW_STYLE(0x3);
const SS_CENTER: WINDOW_STYLE = WINDOW_STYLE(0x1);

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

/// Mở cửa sổ Thông tin, hoặc đưa cửa sổ đang mở ra trước.
pub fn show(owner: HWND) {
    let open = current();
    if !open.is_invalid() {
        winutil::bring_to_front(open);
        return;
    }
    let Some(hwnd) = winutil::create_popup(owner, CLASS, lang::tr(Msg::AboutTitle), wndproc) else {
        return;
    };
    // SAFETY: hwnd vừa tạo ở luồng này.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let font = winutil::system_font(dpi);
    FONT.with(|f| f.set(font));
    OPEN.with(|o| o.set(hwnd.0 as isize));

    let child = |class, text: &str, style, rect, id| {
        winutil::create_child(hwnd, font, dpi, class, text, style, rect, id)
    };
    let icon = child(w!("STATIC"), "", SS_ICON, (136, 16, 48, 48), -1);
    if let Some(i) = winutil::load_app_icon(winutil::scale(48, dpi)) {
        // SAFETY: biểu tượng dùng chung, sống suốt tiến trình; icon là STATIC của cửa sổ này.
        unsafe {
            SendMessageW(icon, STM_SETICON, Some(WPARAM(i.0 as usize)), None);
        }
    }
    child(w!("STATIC"), "darkshot", SS_CENTER, (12, 72, 296, 20), -1);
    child(
        w!("STATIC"),
        &about_version(lang::current(), env!("CARGO_PKG_VERSION")),
        SS_CENTER,
        (12, 94, 296, 18),
        -1,
    );
    child(
        w!("STATIC"),
        lang::tr(Msg::AboutDescription),
        SS_CENTER,
        (12, 120, 296, 54),
        -1,
    );
    let close = child(
        w!("BUTTON"),
        lang::tr(Msg::BtnClose),
        WS_GROUP | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        (120, 184, 80, 28),
        ID_CLOSE,
    );
    winutil::finish_popup(hwnd, 320, 224, dpi);
    // SAFETY: nút vừa tạo trong cửa sổ này.
    unsafe {
        let _ = SetFocus(Some(close));
    }
}

/// Đóng cửa sổ nếu đang mở; không có thì không làm gì.
pub fn close() {
    let h = current();
    if !h.is_invalid() {
        // SAFETY: h là cửa sổ Thông tin còn sống của luồng này; dọn dẹp ở WM_NCDESTROY.
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
        WM_CTLCOLORSTATIC => {
            // Nhãn trùng màu nền cửa sổ (mặc định của cửa sổ thường là nền trắng).
            // SAFETY: hdc do hệ thống cấp cho lần vẽ này; chổi màu hệ thống không phải giải phóng.
            unsafe {
                SetBkMode(HDC(wp.0 as *mut std::ffi::c_void), TRANSPARENT);
                LRESULT(GetSysColorBrush(COLOR_BTNFACE).0 as isize)
            }
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
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored about_window`.
    #[test]
    #[ignore]
    fn about_window_opens_and_closes() {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, hủy ở cuối.
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot-about-test-parent");
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
        let find = || unsafe { FindWindowW(w!("darkshot-about"), None) };
        let mut counts = Vec::new();
        for _ in 0..3 {
            show(parent);
            let first = find().expect("window not created");
            show(parent);
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
