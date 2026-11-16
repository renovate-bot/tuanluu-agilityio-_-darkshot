//! Giao diện tối dùng chung: thanh tiêu đề, menu, điều khiển chuẩn, chổi và màu vẽ.

use darkshot_core::theme::Rgb;
use std::ffi::c_void;
use std::sync::Mutex;
use windows::core::{w, PCSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LRESULT};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, SetBkColor, SetBkMode, SetTextColor, HBRUSH, HDC, OPAQUE,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Controls::SetWindowTheme;

/// Thanh tiêu đề tối. Thuộc tính 20 (Windows 10 20H1 trở lên, 11), lỗi thì thử 19 (bản cũ hơn); không có thì bỏ qua.
pub fn apply_dark_title_bar(hwnd: HWND) {
    let on: i32 = 1;
    for attr in [20, 19] {
        // SAFETY: on là biến cục bộ 4 byte sống suốt lời gọi; hwnd là cửa sổ của luồng này.
        let r = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(attr),
                &on as *const i32 as *const c_void,
                4,
            )
        };
        if r.is_ok() {
            return;
        }
    }
}

/// Menu của tiến trình theo chế độ tối (uxtheme ordinal 135 `SetPreferredAppMode`, 136 `FlushMenuThemes`).
/// Thiếu hàm thì bỏ qua. Gọi một lần trước khi tạo cửa sổ.
pub fn enable_dark_menus() {
    // SAFETY: nạp uxtheme.dll của hệ thống; hàm theo số thứ tự được kiểm None trước khi gọi,
    // và chữ ký (int) -> int / () -> void khớp với hàm không công khai tương ứng.
    unsafe {
        let module = match GetModuleHandleW(w!("uxtheme.dll")) {
            Ok(m) => m,
            Err(_) => match LoadLibraryW(w!("uxtheme.dll")) {
                Ok(m) => m,
                Err(_) => return,
            },
        };
        let ordinal = |n: usize| PCSTR(std::ptr::without_provenance::<u8>(n));
        let Some(set_mode) = GetProcAddress(module, ordinal(135)) else {
            return;
        };
        let set_mode: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(set_mode);
        // 2 = ForceDark.
        set_mode(2);
        if let Some(flush) = GetProcAddress(module, ordinal(136)) {
            let flush: unsafe extern "system" fn() = std::mem::transmute(flush);
            flush();
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // Cfd do Task 30 dùng cho hộp chọn.
pub enum DarkClass {
    /// `DarkMode_Explorer`: nút, ô chữ, thanh cuộn.
    Explorer,
    /// `DarkMode_CFD`: hộp chọn.
    Cfd,
}

/// Cho điều khiển chuẩn dùng giao diện tối của hệ thống; không có thì không đổi gì.
pub fn set_dark_control_theme(hwnd: HWND, class: DarkClass) {
    let name = match class {
        DarkClass::Explorer => w!("DarkMode_Explorer"),
        DarkClass::Cfd => w!("DarkMode_CFD"),
    };
    // SAFETY: hwnd là cửa sổ của luồng này; tên là chuỗi hằng kết thúc bằng 0.
    unsafe {
        let _ = SetWindowTheme(hwnd, name, None);
    }
}

/// Chổi đặc dùng chung, tạo lười một lần cho mỗi màu và sống suốt tiến trình (không bao giờ xóa).
static BRUSHES: Mutex<Vec<(u32, isize)>> = Mutex::new(Vec::new());

pub fn brush(c: Rgb) -> HBRUSH {
    let key = c.colorref();
    let Ok(mut cache) = BRUSHES.lock() else {
        // SAFETY: tạo chổi mới; chỉ xảy ra khi khóa bị hỏng, chấp nhận rò một chổi.
        return unsafe { CreateSolidBrush(COLORREF(key)) };
    };
    if let Some((_, h)) = cache.iter().find(|(k, _)| *k == key) {
        return HBRUSH(*h as *mut c_void);
    }
    // SAFETY: tạo chổi đặc; lưu lại để dùng mãi.
    let h = unsafe { CreateSolidBrush(COLORREF(key)) };
    cache.push((key, h.0 as isize));
    h
}

pub fn colorref(c: Rgb) -> COLORREF {
    COLORREF(c.colorref())
}

/// Xử lý `WM_CTLCOLOR*`: đặt màu chữ và nền cho `hdc`, trả chổi nền dùng chung.
pub fn ctl_color(hdc: HDC, fg: Rgb, bg: Rgb) -> LRESULT {
    // SAFETY: hdc do hệ thống cấp cho lần vẽ này.
    unsafe {
        SetTextColor(hdc, colorref(fg));
        SetBkColor(hdc, colorref(bg));
        SetBkMode(hdc, OPAQUE);
    }
    LRESULT(brush(bg).0 as isize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::i18n::HelpKeys;
    use darkshot_core::theme::{BG, TEXT};
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW,
        RegisterClassW, TranslateMessage, MSG, PM_REMOVE, WINDOW_EX_STYLE, WNDCLASSW, WS_CAPTION,
    };

    #[test]
    fn theme_brush_is_cached() {
        assert_eq!(brush(BG), brush(BG));
        assert_ne!(brush(BG), brush(TEXT));
    }

    unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: chuyển nguyên tham số.
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    fn gdi_count() -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    }

    fn pump() {
        let mut msg = MSG::default();
        // SAFETY: bơm thông điệp còn lại của luồng kiểm thử, không chờ.
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Mở rồi đóng Thông tin và Trợ giúp 20 lần: số đối tượng GDI không tăng.
    #[test]
    fn about_help_no_gdi_leak() {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, hủy ở cuối.
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot-theme-test-parent");
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
        let keys = HelpKeys {
            region: Some("PrintScreen".into()),
            save_full: None,
            upload_full: Some("Ctrl+PrintScreen".into()),
        };
        let mut counts = Vec::new();
        for _ in 0..20 {
            crate::about::show(parent);
            crate::help::show(parent, &keys);
            pump();
            crate::about::close();
            crate::help::close();
            pump();
            counts.push(gdi_count());
        }
        assert!(
            counts[19] <= counts[0],
            "GDI objects grew across opens: {counts:?}"
        );
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
    }
}
