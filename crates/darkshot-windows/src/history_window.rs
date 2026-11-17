//! Cửa sổ Lịch sử ảnh chụp. Task 55 dựng cửa sổ thật; tới lúc đó `show` không làm gì.

use windows::Win32::Foundation::HWND;

/// Mở (hoặc đưa ra trước) cửa sổ Lịch sử. Hiện là chỗ giữ: Task 55 thay bằng cửa sổ thật.
pub fn show(_owner: HWND) {}

#[cfg(test)]
mod tests {
    use super::*;

    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowExW, RegisterClassW,
        WINDOW_EX_STYLE, WNDCLASSW, WS_CAPTION,
    };

    fn rect(l: i32, t: i32, r: i32, b: i32) -> RECT {
        RECT {
            left: l,
            top: t,
            right: r,
            bottom: b,
        }
    }

    #[test]
    fn default_and_min_sizes_scale_with_dpi() {
        assert_eq!(default_client_size(96), (960, 640));
        assert_eq!(default_client_size(120), (1200, 800));
        assert_eq!(default_client_size(144), (1440, 960));
        assert_eq!(default_client_size(192), (1920, 1280));
        assert_eq!(min_client_size(96), (760, 520));
        assert_eq!(min_client_size(144), (1140, 780));
        assert_eq!(min_client_size(192), (1520, 1040));
        // DPI 0 (không hỏi được) coi như 96.
        assert_eq!(default_client_size(0), (960, 640));
        assert_eq!(min_client_size(0), (760, 520));
    }

    #[test]
    fn placement_centers_on_monitor_under_cursor() {
        let m1 = MonitorArea {
            bounds: rect(0, 0, 1920, 1080),
            work: rect(0, 0, 1920, 1040),
        };
        let m2 = MonitorArea {
            bounds: rect(1920, -200, 4480, 1240),
            work: rect(1920, -200, 4480, 1200),
        };
        let mons = [m1, m2];
        // Con trỏ ở màn hình 2: giữa vùng làm việc của nó.
        assert_eq!(
            place_window((2500, 100), &mons, (1000, 700)),
            (1920 + 780, -200 + 350, 1000, 700)
        );
        // Con trỏ ở màn hình 1.
        assert_eq!(
            place_window((10, 10), &mons, (1000, 700)),
            (460, 170, 1000, 700)
        );
        // Con trỏ ngoài mọi màn hình: màn hình gần nhất (màn hình 1).
        assert_eq!(
            place_window((-500, 500), &mons, (1000, 700)),
            (460, 170, 1000, 700)
        );
        // Gần màn hình 2 hơn.
        assert_eq!(
            place_window((5000, 0), &mons, (1000, 700)),
            (1920 + 780, -200 + 350, 1000, 700)
        );
        // Lớn hơn vùng làm việc: thu về vừa vùng làm việc.
        assert_eq!(
            place_window((10, 10), &mons, (2500, 1200)),
            (0, 0, 1920, 1040)
        );
        // Không có danh sách màn hình: đặt ở (100, 100), giữ cỡ.
        assert_eq!(place_window((0, 0), &[], (800, 600)), (100, 100, 800, 600));
    }

    unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: chuyển nguyên tham số.
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    fn gdi_count() -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    }

    fn count_windows() -> usize {
        let mut n = 0;
        let mut after = None;
        // SAFETY: chỉ dò cửa sổ cấp cao theo tên lớp.
        while let Ok(h) = unsafe { FindWindowExW(None, after, w!("darkshot-history"), None) } {
            n += 1;
            after = Some(h);
        }
        n
    }

    /// Mở, mở lần hai (chỉ đưa ra trước), đóng; 20 vòng, số đối tượng GDI không tăng dần.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored history_window`.
    #[test]
    #[ignore]
    fn history_window_opens_and_closes_without_leaks() {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, hủy ở cuối.
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot-history-test-parent");
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
        let mut counts = Vec::new();
        for _ in 0..20 {
            show(parent);
            assert!(is_open());
            let first = hwnd();
            assert_ne!(first, 0);
            assert_eq!(count_windows(), 1);
            show(parent);
            assert_eq!(hwnd(), first, "second show must reuse the window");
            assert_eq!(count_windows(), 1, "must stay a single window");
            close_if_open();
            assert!(!is_open());
            assert_eq!(hwnd(), 0);
            assert_eq!(count_windows(), 0, "window must be gone after close");
            counts.push(gdi_count());
        }
        close_if_open(); // không có cửa sổ: không làm gì
        assert!(
            counts[19] <= counts[1],
            "GDI objects grew across opens: {counts:?}"
        );
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
    }
}
