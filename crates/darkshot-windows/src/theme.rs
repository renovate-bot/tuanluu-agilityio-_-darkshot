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

/// Loại nút bấm: thường (nền xám) hoặc chính (OK, Đóng: nền màu nhấn, chữ trắng đậm).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Normal,
    Primary,
}

/// Trạng thái của một điều khiển lúc vẽ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlState {
    pub hot: bool,
    pub pressed: bool,
    pub focused: bool,
    pub enabled: bool,
    pub checked: bool,
}

/// Bộ màu vẽ: nền, viền, chữ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub fill: Rgb,
    pub border: Rgb,
    pub text: Rgb,
}

/// Màu nút bấm theo loại và trạng thái.
pub fn button_colors(kind: ButtonKind, s: ControlState) -> Colors {
    let _ = (kind, s);
    todo!()
}

/// Hộp ô tích: tắt viền `CHECK_BORDER`, bật nền và viền `ACCENT_BUTTON`; chữ (nằm trên nền khung) `TEXT`
/// hoặc `TEXT_DISABLED`.
pub fn check_colors(s: ControlState) -> Colors {
    let _ = s;
    todo!()
}

/// Vòng nút chọn: bật thì viền và chấm (`fill`) `ACCENT`; tắt viền `CHECK_BORDER`.
pub fn radio_colors(s: ControlState) -> Colors {
    let _ = s;
    todo!()
}

/// Tab: chọn chữ `TEXT`, gạch dưới (`border`) `ACCENT`; không chọn chữ `TEXT_MUTED`; khi trỏ nền
/// `TAB_HOVER` và chữ `TEXT`.
pub fn tab_colors(selected: bool, hot: bool) -> Colors {
    let _ = (selected, hot);
    todo!()
}

/// Màu viền ô nhập và độ dày (DIP) của gạch dưới: 2 khi có tiêu điểm, 1 khi không.
pub fn edit_border(focused: bool, enabled: bool) -> (Rgb, u32) {
    let _ = (focused, enabled);
    todo!()
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

    use darkshot_core::theme::{
        ACCENT, ACCENT_BUTTON, ACCENT_BUTTON_HOVER, ACCENT_BUTTON_PRESSED, BUTTON, BUTTON_BORDER,
        BUTTON_HOVER, BUTTON_PRESSED, CHECK_BORDER, INPUT_BORDER, SURFACE, TAB_HOVER,
        TEXT_DISABLED, TEXT_MUTED, TEXT_PAIRS, WHITE,
    };

    fn st(hot: bool, pressed: bool, focused: bool, enabled: bool, checked: bool) -> ControlState {
        ControlState {
            hot,
            pressed,
            focused,
            enabled,
            checked,
        }
    }

    const NORMAL: ControlState = ControlState {
        hot: false,
        pressed: false,
        focused: false,
        enabled: true,
        checked: false,
    };

    /// Mọi tổ hợp trạng thái.
    fn all_states() -> Vec<ControlState> {
        let mut v = Vec::new();
        for bits in 0..32u8 {
            let b = |i: u8| bits & (1 << i) != 0;
            v.push(st(b(0), b(1), b(2), b(3), b(4)));
        }
        v
    }

    #[test]
    fn primary_button_colors() {
        let c = button_colors(ButtonKind::Primary, NORMAL);
        assert_eq!((c.fill, c.text), (ACCENT_BUTTON, WHITE));
        let hot = button_colors(
            ButtonKind::Primary,
            ControlState {
                hot: true,
                ..NORMAL
            },
        );
        assert_eq!(hot.fill, ACCENT_BUTTON_HOVER);
        let pressed = button_colors(
            ButtonKind::Primary,
            ControlState {
                hot: true,
                pressed: true,
                ..NORMAL
            },
        );
        assert_eq!(pressed.fill, ACCENT_BUTTON_PRESSED);
        let off = button_colors(
            ButtonKind::Primary,
            ControlState {
                enabled: false,
                ..NORMAL
            },
        );
        assert_eq!(off.text, TEXT_DISABLED);
    }

    #[test]
    fn normal_button_colors() {
        assert_eq!(
            button_colors(ButtonKind::Normal, NORMAL),
            Colors {
                fill: BUTTON,
                border: BUTTON_BORDER,
                text: TEXT
            }
        );
        let hot = button_colors(
            ButtonKind::Normal,
            ControlState {
                hot: true,
                ..NORMAL
            },
        );
        assert_eq!(hot.fill, BUTTON_HOVER);
        let pressed = button_colors(
            ButtonKind::Normal,
            ControlState {
                hot: true,
                pressed: true,
                ..NORMAL
            },
        );
        assert_eq!(pressed.fill, BUTTON_PRESSED);
        let off = button_colors(
            ButtonKind::Normal,
            ControlState {
                enabled: false,
                ..NORMAL
            },
        );
        assert_eq!(off.text, TEXT_DISABLED);
    }

    #[test]
    fn check_and_radio_colors() {
        // Ô tích: tắt viền CHECK_BORDER, bật nền ACCENT_BUTTON; chữ TEXT hoặc TEXT_DISABLED.
        let off = check_colors(NORMAL);
        assert_eq!((off.border, off.text), (CHECK_BORDER, TEXT));
        let on = check_colors(ControlState {
            checked: true,
            ..NORMAL
        });
        assert_eq!((on.fill, on.text), (ACCENT_BUTTON, TEXT));
        for checked in [false, true] {
            let dis = check_colors(ControlState {
                checked,
                enabled: false,
                ..NORMAL
            });
            assert_eq!(dis.text, TEXT_DISABLED, "checked={checked}");
            assert_ne!(dis.fill, ACCENT_BUTTON, "checked={checked}");
        }
        // Nút chọn: bật viền và chấm ACCENT; tắt viền CHECK_BORDER.
        let off = radio_colors(NORMAL);
        assert_eq!((off.border, off.text), (CHECK_BORDER, TEXT));
        let on = radio_colors(ControlState {
            checked: true,
            ..NORMAL
        });
        assert_eq!((on.border, on.fill, on.text), (ACCENT, ACCENT, TEXT));
        for checked in [false, true] {
            let dis = radio_colors(ControlState {
                checked,
                enabled: false,
                ..NORMAL
            });
            assert_eq!(dis.text, TEXT_DISABLED, "checked={checked}");
            assert_ne!(dis.border, ACCENT, "checked={checked}");
        }
    }

    #[test]
    fn tab_colors_selected_and_hot() {
        let sel = tab_colors(true, false);
        assert_eq!((sel.text, sel.border, sel.fill), (TEXT, ACCENT, BG));
        let other = tab_colors(false, false);
        assert_eq!((other.text, other.fill), (TEXT_MUTED, BG));
        assert_eq!(tab_colors(false, true).fill, TAB_HOVER);
        assert_eq!(tab_colors(true, true).fill, TAB_HOVER);
        assert_eq!(tab_colors(true, true).border, ACCENT);
    }

    #[test]
    fn edit_border_focus() {
        assert_eq!(edit_border(true, true), (ACCENT, 2));
        assert_eq!(edit_border(false, true), (INPUT_BORDER, 1));
        assert_eq!(edit_border(false, false).1, 1);
    }

    /// Mọi cặp (chữ, nền dưới chữ) có thể vẽ khi điều khiển bật đều nằm trong `TEXT_PAIRS`, nên kiểm thử
    /// tương phản của core bao luôn chúng. Chữ của ô tích và nút chọn nằm trên nền khung (`SURFACE`),
    /// không nằm trên hộp, nên cặp của chúng là (text, SURFACE).
    #[test]
    fn every_drawn_text_pair_is_listed() {
        let listed = |fg: Rgb, bg: Rgb| TEXT_PAIRS.contains(&(fg, bg));
        for s in all_states().into_iter().filter(|s| s.enabled) {
            for kind in [ButtonKind::Normal, ButtonKind::Primary] {
                let c = button_colors(kind, s);
                assert!(listed(c.text, c.fill), "{kind:?} {s:?}: {c:?}");
            }
            let c = check_colors(s);
            assert!(listed(c.text, SURFACE), "check {s:?}: {c:?}");
            let c = radio_colors(s);
            assert!(listed(c.text, SURFACE), "radio {s:?}: {c:?}");
        }
        for selected in [false, true] {
            for hot in [false, true] {
                let c = tab_colors(selected, hot);
                assert!(listed(c.text, c.fill), "tab {selected} {hot}: {c:?}");
            }
        }
    }

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
