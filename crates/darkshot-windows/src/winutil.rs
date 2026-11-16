//! Phần dùng chung của các cửa sổ phụ (Tùy chọn, Thông tin, Trợ giúp).

use std::ffi::c_void;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, GetMonitorInfoW, GetSysColorBrush, MonitorFromPoint, COLOR_BTNFACE, HFONT,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, SystemParametersInfoForDpi};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetCursorPos, GetSystemMetrics, IsChild, IsDialogMessageW, IsIconic,
    LoadImageW, RegisterClassW, SendMessageW, SetForegroundWindow, SetWindowPos, ShowWindow,
    CW_USEDEFAULT, HICON, HMENU, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_SHARED, MSG,
    NONCLIENTMETRICSW, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SPI_GETNONCLIENTMETRICS,
    SWP_NOZORDER, SW_RESTORE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_KEYDOWN, WM_SETFONT,
    WM_SETICON, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_EX_APPWINDOW, WS_EX_CONTROLPARENT, WS_SYSMENU,
    WS_VISIBLE,
};

/// Phông thông báo của hệ thống ở DPI `dpi`. Người gọi phải `DeleteObject` khi xong.
pub fn system_font(dpi: u32) -> Option<HFONT> {
    let mut ncm = NONCLIENTMETRICSW {
        cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };
    // SAFETY: ncm có cbSize đúng; hệ thống điền lfMessageFont.
    unsafe {
        SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS.0,
            ncm.cbSize,
            Some(&mut ncm as *mut _ as *mut c_void),
            0,
            dpi,
        )
        .ok()?;
        let f = CreateFontIndirectW(&ncm.lfMessageFont);
        if f.is_invalid() {
            None
        } else {
            Some(f)
        }
    }
}

/// Đặt cửa sổ cỡ `width` x `height` (pixel vật lý) vào giữa vùng làm việc của màn hình đang có con trỏ.
pub fn center_on_cursor_monitor(hwnd: HWND, width: i32, height: i32) {
    // SAFETY: các lời gọi hệ thống chỉ dùng giá trị cục bộ; hwnd là cửa sổ của luồng này.
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let (x, y) = if GetMonitorInfoW(mon, &mut mi).as_bool() {
            let a = mi.rcWork;
            (
                a.left + ((a.right - a.left) - width) / 2,
                a.top + ((a.bottom - a.top) - height) / 2,
            )
        } else {
            (100, 100)
        };
        let _ = SetWindowPos(hwnd, None, x, y, width, height, SWP_NOZORDER);
    }
}

/// Biểu tượng lớn (thanh tác vụ, Alt+Tab) và nhỏ (thanh tiêu đề) của ứng dụng; `LR_SHARED` nên không phải hủy.
pub fn set_app_icons(hwnd: HWND) {
    // SAFETY: tài nguyên biểu tượng số 1 do build.rs nhúng; hwnd là cửa sổ của luồng này.
    unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            return;
        };
        let res = PCWSTR(std::ptr::without_provenance::<u16>(1));
        for (which, metric_x, metric_y) in [
            (ICON_BIG, SM_CXICON, SM_CYICON),
            (ICON_SMALL, SM_CXSMICON, SM_CYSMICON),
        ] {
            let (cx, cy) = (GetSystemMetrics(metric_x), GetSystemMetrics(metric_y));
            if let Ok(icon) = LoadImageW(Some(hinst.into()), res, IMAGE_ICON, cx, cy, LR_SHARED) {
                SendMessageW(
                    hwnd,
                    WM_SETICON,
                    Some(WPARAM(which as usize)),
                    Some(LPARAM(icon.0 as isize)),
                );
            }
        }
    }
}

/// Quy đổi DIP sang pixel ở `dpi`.
pub fn scale(v: i32, dpi: u32) -> i32 {
    (v * dpi as i32 + 48) / 96
}

/// Đưa cửa sổ ra trước, khôi phục nếu đang thu nhỏ.
pub fn bring_to_front(hwnd: HWND) {
    // SAFETY: hwnd là cửa sổ của luồng này (người gọi giữ HWND chỉ khi cửa sổ còn sống).
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

const POPUP_STYLE: WINDOW_STYLE = WINDOW_STYLE(WS_CAPTION.0 | WS_SYSMENU.0);
const POPUP_EX: WINDOW_EX_STYLE = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0 | WS_EX_CONTROLPARENT.0);

/// Tạo cửa sổ phụ cố định cỡ (chưa hiện): có tiêu đề, nút đóng, biểu tượng darkshot, thuộc `owner`.
pub fn create_popup(
    owner: HWND,
    class: PCWSTR,
    title: &str,
    proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) -> Option<HWND> {
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; đăng ký lại lần hai báo lỗi vô hại.
    let hwnd = unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: hinst.into(),
            lpszClassName: class,
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..Default::default()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            POPUP_EX,
            class,
            &HSTRING::from(title),
            POPUP_STYLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            100,
            100,
            Some(owner),
            None,
            Some(hinst.into()),
            None,
        )
        .ok()?
    };
    set_app_icons(hwnd);
    Some(hwnd)
}

/// Đặt vùng client `client_w` x `client_h` DIP, căn giữa màn hình có con trỏ, hiện và đưa ra trước.
pub fn finish_popup(hwnd: HWND, client_w: i32, client_h: i32, dpi: u32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: scale(client_w, dpi),
        bottom: scale(client_h, dpi),
    };
    // SAFETY: rc là biến cục bộ; hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = AdjustWindowRectExForDpi(&mut rc, POPUP_STYLE, false, POPUP_EX, dpi);
    }
    center_on_cursor_monitor(hwnd, rc.right - rc.left, rc.bottom - rc.top);
    // SAFETY: hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Tạo điều khiển con (tọa độ DIP) và đặt phông. Lỗi thì trả cửa sổ rỗng.
#[allow(clippy::too_many_arguments)]
pub fn create_child(
    parent: HWND,
    font: Option<HFONT>,
    dpi: u32,
    class: PCWSTR,
    text: &str,
    style: WINDOW_STYLE,
    (x, y, w, h): (i32, i32, i32, i32),
    id: i32,
) -> HWND {
    // SAFETY: tạo cửa sổ con của cửa sổ phụ ở luồng chính.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            &HSTRING::from(text),
            WS_CHILD | WS_VISIBLE | style,
            scale(x, dpi),
            scale(y, dpi),
            scale(w, dpi),
            scale(h, dpi),
            Some(parent),
            Some(HMENU(id as usize as *mut c_void)),
            None,
            None,
        )
        .unwrap_or_default()
    };
    if let (Some(f), false) = (font, hwnd.is_invalid()) {
        // SAFETY: phông sống đến khi cửa sổ phụ bị hủy.
        unsafe {
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(f.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }
    hwnd
}

/// Thông điệp `msg` thuộc cửa sổ `hwnd` hoặc con của nó.
pub fn belongs_to(hwnd: HWND, msg: &MSG) -> bool {
    // SAFETY: IsChild chỉ đọc quan hệ cửa sổ.
    msg.hwnd == hwnd || unsafe { IsChild(hwnd, msg.hwnd).as_bool() }
}

/// Phần chung của `pre_translate` của hai cửa sổ thông tin: Enter và Esc đóng cửa sổ (gọi `close`),
/// còn lại (Tab, mũi tên) để `IsDialogMessageW` xử lý. `true` = thông điệp đã được xử lý.
pub fn dialog_pre_translate(hwnd: HWND, msg: &MSG, close: impl FnOnce()) -> bool {
    if hwnd.is_invalid() || !belongs_to(hwnd, msg) {
        return false;
    }
    if msg.message == WM_KEYDOWN
        && (msg.wParam.0 as u16 == VK_ESCAPE.0 || msg.wParam.0 as u16 == VK_RETURN.0)
    {
        close();
        return true;
    }
    // SAFETY: xử lý phím điều hướng của cửa sổ phụ.
    unsafe { IsDialogMessageW(hwnd, msg).as_bool() }
}

/// Biểu tượng ứng dụng cỡ `px` pixel; `LR_SHARED` nên không phải hủy.
pub fn load_app_icon(px: i32) -> Option<HICON> {
    // SAFETY: tài nguyên biểu tượng số 1 do build.rs nhúng.
    unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let res = PCWSTR(std::ptr::without_provenance::<u16>(1));
        LoadImageW(Some(hinst.into()), res, IMAGE_ICON, px, px, LR_SHARED)
            .ok()
            .map(|h| HICON(h.0))
    }
}
