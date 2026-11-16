//! Cửa sổ Thông tin: không chặn, tối đa một cửa sổ.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::MSG;

pub fn show(_owner: HWND) {
    todo!()
}

/// Đóng cửa sổ nếu đang mở; không có thì không làm gì.
pub fn close() {
    todo!()
}

/// `true`: thông điệp đã được xử lý, vòng lặp bỏ qua.
pub fn pre_translate(_msg: &MSG) -> bool {
    todo!()
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
    /// Chạy tay: `cargo test -p shotme-windows -- --ignored about`.
    #[test]
    #[ignore]
    fn about_opens_and_closes() {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, hủy ở cuối.
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("shotme-about-test-parent");
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
        let find = || unsafe { FindWindowW(w!("shotme-about"), None) };
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
