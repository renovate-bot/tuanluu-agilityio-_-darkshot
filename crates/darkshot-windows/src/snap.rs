//! Liệt kê khung các cửa sổ cấp cao để gợi ý khi chọn vùng (chỉ đọc khung, không đọc tiêu đề).

use crate::capture::DpiScope;
use darkshot_core::geometry::{Point, Rect};
use darkshot_core::snap::MAX_WINDOWS;
use std::ffi::c_void;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible,
    GWL_EXSTYLE, WS_EX_TRANSPARENT,
};

/// Khung các cửa sổ cấp cao hiện được, thứ tự Z từ trên xuống, tọa độ MÀN HÌNH, pixel vật lý.
/// Tối đa darkshot_core::snap::MAX_WINDOWS. Lỗi liệt kê → danh sách (có thể rỗng) đã có tới lúc lỗi.
pub fn enumerate_windows() -> Vec<Rect> {
    // Khung DWM và GetWindowRect trả pixel vật lý khi luồng ở PerMonitorV2.
    let _dpi = DpiScope::per_monitor_v2();
    let mut out: Vec<Rect> = Vec::new();
    // SAFETY: EnumWindows đồng bộ; callback chỉ ghi vào `out`, sống đến hết lời gọi này.
    // Lỗi (hoặc callback dừng sớm) bị bỏ qua: giữ những gì đã có.
    unsafe {
        let _ = EnumWindows(
            Some(collect_window),
            LPARAM(&mut out as *mut Vec<Rect> as isize),
        );
    }
    out
}

/// Callback của EnumWindows: thêm khung nếu cửa sổ là ứng viên; trả FALSE khi đã đủ.
unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam trỏ tới `Vec<Rect>` cục bộ của `enumerate_windows`, còn sống trong lời gọi đồng bộ.
    let out = unsafe { &mut *(lparam.0 as *mut Vec<Rect>) };
    if out.len() >= MAX_WINDOWS {
        return BOOL(0);
    }
    if is_candidate(hwnd) {
        if let Some(r) = frame_of(hwnd) {
            out.push(r);
        }
    }
    BOOL::from(out.len() < MAX_WINDOWS)
}

/// Đọc các thuộc tính (không đọc tiêu đề) rồi giao cho `accept`.
fn is_candidate(hwnd: HWND) -> bool {
    // SAFETY: các hàm chỉ đọc thuộc tính của hwnd hệ thống vừa đưa; hwnd đã hủy thì trả FALSE/0.
    let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
    if !visible {
        // Đa số cửa sổ cấp cao đều ẩn: bỏ sớm, kết quả giống `accept(false, ..)`.
        return false;
    }
    // SAFETY: như trên.
    let iconic = unsafe { IsIconic(hwnd) }.as_bool();
    let mut cloaked: u32 = 0;
    // SAFETY: bộ đệm DWORD đúng cỡ cho DWMWA_CLOAKED; lỗi thì coi là không cloaked.
    let cloaked = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        )
    }
    .is_ok()
        && cloaked != 0;
    // SAFETY: chỉ đọc kiểu mở rộng của hwnd.
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    let mut buf = [0u16; 256];
    // SAFETY: bộ đệm cục bộ; hàm ghi tối đa buf.len() ký tự kể cả số 0 cuối.
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    let len = usize::try_from(n).unwrap_or(0).min(buf.len());
    let class = String::from_utf16_lossy(&buf[..len]);
    accept(visible, iconic, cloaked, ex_style, &class)
}

/// Khung cửa sổ: ưu tiên khung DWM (không gồm bóng), lỗi thì GetWindowRect. Rỗng/âm → None.
fn frame_of(hwnd: HWND) -> Option<Rect> {
    let mut rc = RECT::default();
    // SAFETY: bộ đệm RECT đúng cỡ cho DWMWA_EXTENDED_FRAME_BOUNDS.
    let dwm = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rc as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if dwm.is_err() {
        rc = RECT::default();
        // SAFETY: rc là RECT cục bộ.
        unsafe { GetWindowRect(hwnd, &mut rc) }.ok()?;
    }
    rect_from(rc)
}

/// RECT (trái, trên, phải, dưới) → Rect; None khi rỗng, âm hoặc tràn.
fn rect_from(rc: RECT) -> Option<Rect> {
    let w = i64::from(rc.right) - i64::from(rc.left);
    let h = i64::from(rc.bottom) - i64::from(rc.top);
    if w <= 0 || h <= 0 {
        return None;
    }
    Some(Rect {
        x: rc.left,
        y: rc.top,
        w: i32::try_from(w).ok()?,
        h: i32::try_from(h).ok()?,
    })
}

/// Thuần: lớp cửa sổ là nền màn hình (bỏ qua). So khớp không phân biệt hoa thường: "Progman", "WorkerW".
pub fn is_shell_background(class: &str) -> bool {
    class.eq_ignore_ascii_case("Progman") || class.eq_ignore_ascii_case("WorkerW")
}

/// Thuần: quyết định một cửa sổ có là ứng viên không từ các thuộc tính đã đọc.
pub fn accept(visible: bool, iconic: bool, cloaked: bool, ex_style: u32, class: &str) -> bool {
    visible
        && !iconic
        && !cloaked
        && ex_style & WS_EX_TRANSPARENT.0 == 0
        && !is_shell_background(class)
}

/// Đổi khung từ tọa độ màn hình sang tọa độ ảnh của màn hình ảo có gốc `origin` (trừ bão hòa).
pub fn windows_to_image(windows: &[Rect], origin: Point) -> Vec<Rect> {
    windows
        .iter()
        .map(|r| Rect {
            x: r.x.saturating_sub(origin.x),
            y: r.y.saturating_sub(origin.y),
            ..*r
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::w;
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, ShowWindow,
        UnregisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        SetLayeredWindowAttributes, LWA_ALPHA, LWA_COLORKEY, WS_EX_LAYERED,
    };

    const TRANSPARENT: u32 = WS_EX_TRANSPARENT.0;
    const LAYERED: u32 = WS_EX_LAYERED.0;
    const TOOL: u32 = WS_EX_TOOLWINDOW.0;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn shell_background_classes() {
        assert!(is_shell_background("Progman"));
        assert!(is_shell_background("progman"));
        assert!(is_shell_background("WorkerW"));
        assert!(is_shell_background("WORKERW"));
        assert!(!is_shell_background("Chrome_WidgetWin_1"));
        assert!(!is_shell_background(""));
    }

    #[test]
    fn accept_table() {
        let cls = "Notepad";
        // Bình thường.
        assert!(accept(true, false, false, false, 0, cls));
        // WS_EX_TOOLWINDOW đơn thuần vẫn nhận.
        assert!(accept(true, false, false, false, TOOL, cls));
        // Ẩn.
        assert!(!accept(false, false, false, false, 0, cls));
        // Thu nhỏ.
        assert!(!accept(true, true, false, false, 0, cls));
        // Cloaked.
        assert!(!accept(true, false, true, false, 0, cls));
        // Lớp phủ vô hình (WS_EX_LAYERED, độ mờ 0).
        assert!(!accept(true, false, false, true, LAYERED, cls));
        assert!(accept(true, false, false, false, LAYERED, cls));
        // Trong suốt với chuột.
        assert!(!accept(true, false, false, false, TRANSPARENT, cls));
        assert!(!accept(true, false, false, false, TRANSPARENT | TOOL, cls));
        // Nền màn hình.
        assert!(!accept(true, false, false, false, 0, "Progman"));
        assert!(!accept(true, false, false, false, 0, "WorkerW"));
    }

    #[test]
    fn invisible_layered_table() {
        let alpha = LWA_ALPHA.0;
        let key = LWA_COLORKEY.0;
        // Không phải cửa sổ layered: không đọc, không loại.
        assert!(!is_invisible_layered(0, Some((0, alpha))));
        // Layered, độ mờ 0 qua LWA_ALPHA: loại.
        assert!(is_invisible_layered(LAYERED, Some((0, alpha))));
        assert!(is_invisible_layered(LAYERED, Some((0, alpha | key))));
        // Độ mờ khác 0, hoặc không có LWA_ALPHA (chỉ màu khóa): giữ.
        assert!(!is_invisible_layered(LAYERED, Some((1, alpha))));
        assert!(!is_invisible_layered(LAYERED, Some((255, alpha))));
        assert!(!is_invisible_layered(LAYERED, Some((0, key))));
        assert!(!is_invisible_layered(LAYERED, Some((0, 0))));
        // Lỗi đọc (ví dụ cửa sổ dùng UpdateLayeredWindow): giữ.
        assert!(!is_invisible_layered(LAYERED, None));
    }

    #[test]
    fn screenshot_windows_none_by_default() {
        let origin = Point { x: -1920, y: -200 };
        let got = windows_to_image(&[r(-1920, -200, 800, 600), r(100, 50, 30, 40)], origin);
        assert_eq!(got, vec![r(0, 0, 800, 600), r(2020, 250, 30, 40)]);
        // Trừ bão hòa: không tràn.
        let got = windows_to_image(&[r(i32::MAX, i32::MIN, 10, 10)], Point { x: -5, y: 5 });
        assert_eq!(got, vec![r(i32::MAX, i32::MIN, 10, 10)]);
        assert!(windows_to_image(&[], origin).is_empty());
    }

    unsafe extern "system" fn test_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: chuyển nguyên tham số hệ thống đưa vào.
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    struct ClassGuard(windows::Win32::Foundation::HINSTANCE, windows::core::PCWSTR);
    impl Drop for ClassGuard {
        fn drop(&mut self) {
            // SAFETY: hủy đăng ký lớp do kiểm thử đăng ký; cửa sổ của lớp đã bị hủy trước (drop ngược thứ tự).
            unsafe {
                let _ = UnregisterClassW(self.1, Some(self.0));
            }
        }
    }

    struct WindowGuard(HWND);
    impl Drop for WindowGuard {
        fn drop(&mut self) {
            // SAFETY: hủy cửa sổ do luồng này tạo.
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    #[test]
    fn enumerate_finds_own_visible_window() {
        // Tạo cửa sổ theo pixel vật lý, như ứng dụng thật (manifest PerMonitorV2).
        let _dpi = crate::capture::DpiScope::per_monitor_v2();
        // SAFETY: lấy handle của chính tiến trình.
        let inst = unsafe { GetModuleHandleW(None) }.unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(test_proc),
            hInstance: inst.into(),
            lpszClassName: w!("DarkshotSnapTestPopup"),
            ..Default::default()
        };
        // SAFETY: wc hợp lệ, tên lớp là hằng tĩnh.
        let atom = unsafe { RegisterClassW(&wc) };
        assert_ne!(atom, 0);
        let _class = ClassGuard(inst.into(), w!("DarkshotSnapTestPopup"));
        // SAFETY: lớp vừa đăng ký; không có cửa sổ cha.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("DarkshotSnapTestPopup"),
                None,
                WS_POPUP | WS_VISIBLE,
                100,
                100,
                300,
                200,
                None,
                None,
                Some(inst.into()),
                None,
            )
        }
        .unwrap();
        let _win = WindowGuard(hwnd);
        let want = r(100, 100, 300, 200);

        let start = std::time::Instant::now();
        let found = enumerate_windows();
        let took = start.elapsed();
        println!("enumerate_windows: {} khung, {:?}", found.len(), took);
        assert!(found.contains(&want), "{found:?}");
        assert!(found.len() <= darkshot_core::snap::MAX_WINDOWS);
        assert!(found.iter().all(|f| f.w > 0 && f.h > 0));

        // SAFETY: cửa sổ của luồng này.
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        assert!(!enumerate_windows().contains(&want));
    }

    #[test]
    fn enumerate_skips_invisible_layered_window() {
        let _dpi = crate::capture::DpiScope::per_monitor_v2();
        // SAFETY: lấy handle của chính tiến trình.
        let inst = unsafe { GetModuleHandleW(None) }.unwrap();
        // Lớp riêng: kiểm thử chạy song song với `enumerate_finds_own_visible_window`.
        let class = w!("DarkshotSnapTestLayered");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(test_proc),
            hInstance: inst.into(),
            lpszClassName: class,
            ..Default::default()
        };
        // SAFETY: wc hợp lệ, tên lớp là hằng tĩnh.
        let atom = unsafe { RegisterClassW(&wc) };
        assert_ne!(atom, 0);
        let _class = ClassGuard(inst.into(), class);
        let make = |x: i32, alpha: u8| {
            // SAFETY: lớp vừa đăng ký; không có cửa sổ cha.
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                    class,
                    None,
                    WS_POPUP,
                    x,
                    130,
                    210,
                    170,
                    None,
                    None,
                    Some(inst.into()),
                    None,
                )
            }
            .unwrap();
            let guard = WindowGuard(hwnd);
            // SAFETY: cửa sổ layered của luồng này.
            unsafe {
                SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA).unwrap();
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            guard
        };
        let _clear = make(130, 0);
        let _solid = make(370, 255);
        let found = enumerate_windows();
        assert!(!found.contains(&r(130, 130, 210, 170)), "{found:?}");
        assert!(found.contains(&r(370, 130, 210, 170)), "{found:?}");
    }

    #[test]
    #[ignore = "đo thời gian; chạy tay: cargo test -p darkshot-windows -- --ignored enumerate_timing --test-threads=1 --nocapture"]
    fn enumerate_timing() {
        let _ = enumerate_windows();
        let mut best = std::time::Duration::MAX;
        let mut n = 0;
        for _ in 0..20 {
            let start = std::time::Instant::now();
            n = enumerate_windows().len();
            best = best.min(start.elapsed());
        }
        println!("enumerate_windows: {n} khung, tốt nhất {best:?}");
        assert!(best < std::time::Duration::from_millis(10), "{best:?}");
    }
}
