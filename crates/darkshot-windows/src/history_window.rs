//! Cửa sổ Lịch sử ảnh chụp: không chặn, tối đa một cửa sổ, đổi cỡ được, vẽ theo DPI của cửa sổ.
//! Thông điệp do vòng lặp của ứng dụng phân phối; `pre_translate` lo phím Esc.
//! Khung (Task 55): chỉ lớp cửa sổ và nền `BG`; nội dung do task sau vẽ.

use crate::history_service;
use crate::lang;
use crate::theme;
use crate::winutil::{self, scale};
use darkshot_core::i18n::Msg;
use darkshot_core::theme::BG;
use std::cell::Cell;
use std::ffi::c_void;
use windows::core::{w, BOOL, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, InvalidateRect, MonitorFromPoint, HDC, HMONITOR,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, IsWindow,
    RegisterClassW, SetForegroundWindow, SetWindowPos, ShowWindow, MINMAXINFO, MSG, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_DPICHANGED, WM_ERASEBKGND,
    WM_GETMINMAXINFO, WM_KEYDOWN, WM_NCDESTROY, WNDCLASSW, WS_CAPTION, WS_EX_APPWINDOW,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
};

const CLASS: PCWSTR = w!("darkshot-history");
/// Cỡ vùng client mặc định và tối thiểu (DIP).
const DEFAULT_W: i32 = 960;
const DEFAULT_H: i32 = 640;
const MIN_W: i32 = 760;
const MIN_H: i32 = 520;
const STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_CAPTION.0 | WS_SYSMENU.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0,
);
const EX_STYLE: WINDOW_EX_STYLE = WS_EX_APPWINDOW;

thread_local! {
    /// HWND của cửa sổ đang mở (0 = không có).
    static OPEN: Cell<isize> = const { Cell::new(0) };
}

/// DPI 0 (không hỏi được) coi như 96.
fn norm_dpi(dpi: u32) -> u32 {
    if dpi == 0 {
        96
    } else {
        dpi
    }
}

/// Cỡ vùng client mặc định (pixel) ở `dpi`.
pub fn default_client_size(dpi: u32) -> (i32, i32) {
    let d = norm_dpi(dpi);
    (scale(DEFAULT_W, d), scale(DEFAULT_H, d))
}

/// Cỡ vùng client nhỏ nhất (pixel) ở `dpi`.
pub fn min_client_size(dpi: u32) -> (i32, i32) {
    let d = norm_dpi(dpi);
    (scale(MIN_W, d), scale(MIN_H, d))
}

/// Một màn hình: toàn bộ và vùng làm việc (trừ thanh tác vụ), pixel màn hình ảo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonitorArea {
    pub bounds: RECT,
    pub work: RECT,
}

/// Bình phương khoảng cách từ điểm tới hình chữ nhật (0 khi nằm trong).
fn dist2(p: (i32, i32), r: &RECT) -> i64 {
    let dx = if p.0 < r.left {
        r.left - p.0
    } else if p.0 >= r.right {
        p.0 - r.right + 1
    } else {
        0
    };
    let dy = if p.1 < r.top {
        r.top - p.1
    } else if p.1 >= r.bottom {
        p.1 - r.bottom + 1
    } else {
        0
    };
    i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
}

/// Vị trí và cỡ cửa sổ `(x, y, w, h)`: giữa vùng làm việc của màn hình chứa con trỏ (không có thì
/// màn hình gần nhất), thu nhỏ cho vừa vùng làm việc. Danh sách rỗng: đặt ở (100, 100).
pub fn place_window(
    cursor: (i32, i32),
    monitors: &[MonitorArea],
    size: (i32, i32),
) -> (i32, i32, i32, i32) {
    let Some(m) = monitors.iter().min_by_key(|m| dist2(cursor, &m.bounds)) else {
        return (100, 100, size.0, size.1);
    };
    let a = m.work;
    let (aw, ah) = ((a.right - a.left).max(0), (a.bottom - a.top).max(0));
    let (w, h) = (size.0.min(aw), size.1.min(ah));
    (a.left + (aw - w) / 2, a.top + (ah - h) / 2, w, h)
}

/// Cỡ cửa sổ ngoài (pixel) cho vùng client `w` x `h` ở `dpi`.
fn window_size(w: i32, h: i32, dpi: u32) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    // SAFETY: rc là biến cục bộ; hàm chỉ tính toán.
    unsafe {
        let _ = AdjustWindowRectExForDpi(&mut rc, STYLE, false, EX_STYLE, dpi);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

unsafe extern "system" fn collect_monitor(
    m: HMONITOR,
    _dc: HDC,
    _rc: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `data` trỏ tới Vec<MonitorArea> còn sống trong `monitors`; mi có cbSize đúng.
    unsafe {
        if GetMonitorInfoW(m, &mut mi).as_bool() {
            (*(data.0 as *mut Vec<MonitorArea>)).push(MonitorArea {
                bounds: mi.rcMonitor,
                work: mi.rcWork,
            });
        }
    }
    BOOL(1)
}

/// Mọi màn hình đang gắn.
fn monitors() -> Vec<MonitorArea> {
    let mut out: Vec<MonitorArea> = Vec::new();
    // SAFETY: callback chỉ ghi vào `out`, sống đến hết lời gọi đồng bộ này.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut out as *mut Vec<MonitorArea> as isize),
        );
    }
    out
}

/// DPI của màn hình chứa (hoặc gần) điểm `pt`.
fn dpi_at(pt: POINT) -> u32 {
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: các tham số ra là biến cục bộ.
    let ok = unsafe {
        let m = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_ok()
    };
    if ok {
        norm_dpi(x)
    } else {
        96
    }
}

fn current() -> HWND {
    let h = OPEN.with(|o| o.get());
    let hwnd = HWND(h as *mut c_void);
    // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
    if h != 0 && unsafe { IsWindow(Some(hwnd)).as_bool() } {
        hwnd
    } else {
        HWND::default()
    }
}

/// Cửa sổ Lịch sử đang mở. Hợp đồng cho các task sau của đợt 7 (hiện chỉ kiểm thử dùng).
#[allow(dead_code)]
pub fn is_open() -> bool {
    !current().is_invalid()
}

/// HWND của cửa sổ đang mở (0 = không có). Hợp đồng cho các task sau (hiện chỉ kiểm thử dùng).
#[allow(dead_code)]
pub fn hwnd() -> isize {
    current().0 as isize
}

/// Mở cửa sổ Lịch sử, hoặc đưa cửa sổ đang mở ra trước.
pub fn show(owner: HWND) {
    let open = current();
    if !open.is_invalid() {
        winutil::bring_to_front(open);
        return;
    }
    let mut pt = POINT::default();
    // SAFETY: pt là biến cục bộ; lỗi thì giữ (0, 0).
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    // Tạo thẳng trên màn hình có con trỏ, theo DPI của nó: không có WM_DPICHANGED lúc mở.
    let dpi = dpi_at(pt);
    let (cw, ch) = default_client_size(dpi);
    let (ww, wh) = window_size(cw, ch, dpi);
    let (x, y, w, h) = place_window((pt.x, pt.y), &monitors(), (ww, wh));
    let title = HSTRING::from(lang::tr(Msg::HistoryTitle));
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; đăng ký lại lần hai báo lỗi vô hại.
    let created = unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            return;
        };
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: CLASS,
            hbrBackground: theme::brush(BG),
            ..Default::default()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            EX_STYLE,
            CLASS,
            &title,
            STYLE,
            x,
            y,
            w,
            h,
            Some(owner),
            None,
            Some(hinst.into()),
            None,
        )
    };
    let Ok(hwnd) = created else {
        return;
    };
    OPEN.with(|o| o.set(hwnd.0 as isize));
    history_service::set_window(hwnd.0 as isize);
    winutil::set_app_icons(hwnd);
    theme::apply_dark_title_bar(hwnd);
    // SAFETY: hwnd vừa tạo ở luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Đóng cửa sổ nếu đang mở; không có thì không làm gì.
pub fn close_if_open() {
    let h = current();
    if !h.is_invalid() {
        // SAFETY: h là cửa sổ Lịch sử còn sống của luồng này; dọn dẹp ở WM_NCDESTROY.
        unsafe {
            let _ = DestroyWindow(h);
        }
    }
}

/// `true`: thông điệp đã được xử lý, vòng lặp bỏ qua. Esc trong cửa sổ (hoặc con của nó) đóng cửa sổ.
pub fn pre_translate(msg: &MSG) -> bool {
    let h = current();
    if h.is_invalid() || !winutil::belongs_to(h, msg) {
        return false;
    }
    if msg.message == WM_KEYDOWN && msg.wParam.0 as u16 == VK_ESCAPE.0 {
        close_if_open();
        return true;
    }
    false
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            // SAFETY: hủy chính cửa sổ này; dọn dẹp ở WM_NCDESTROY.
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            let mut rc = RECT::default();
            // SAFETY: rc là biến cục bộ; wp là HDC hệ thống cấp cho lần xóa nền này.
            unsafe {
                let _ = GetClientRect(hwnd, &mut rc);
            }
            theme::fill(HDC(wp.0 as *mut c_void), &rc, BG);
            LRESULT(1)
        }
        WM_GETMINMAXINFO => {
            // SAFETY: hwnd là cửa sổ này (có thể đang được tạo); chỉ hỏi DPI.
            let dpi = norm_dpi(unsafe { GetDpiForWindow(hwnd) });
            let (mw, mh) = min_client_size(dpi);
            let (ww, wh) = window_size(mw, mh, dpi);
            // SAFETY: với WM_GETMINMAXINFO, lp trỏ tới MINMAXINFO hợp lệ suốt lời gọi.
            if let Some(mmi) = unsafe { (lp.0 as *mut MINMAXINFO).as_mut() } {
                mmi.ptMinTrackSize.x = ww;
                mmi.ptMinTrackSize.y = wh;
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            // Tài liệu WM_DPICHANGED: dùng đúng RECT đề nghị trong lp.
            // SAFETY: lp trỏ tới RECT hợp lệ suốt lời gọi.
            if let Some(r) = unsafe { (lp.0 as *const RECT).as_ref() } {
                // SAFETY: đổi vị trí và cỡ chính cửa sổ này.
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    let _ = InvalidateRect(Some(hwnd), None, true);
                }
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // Quên handle, dù đóng bằng đường nào.
            if OPEN.with(|o| o.get()) == hwnd.0 as isize {
                OPEN.with(|o| o.set(0));
                history_service::set_window(0);
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

    fn test_parent(class: PCWSTR) -> HWND {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, bên gọi hủy ở cuối.
        unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
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
        }
    }

    /// Chạy vòng thông điệp của luồng này tới khi `done()` hoặc hết `ms` mili giây.
    fn pump_until(ms: u64, done: impl Fn() -> bool) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, PM_REMOVE,
        };
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        loop {
            let mut msg = MSG::default();
            // SAFETY: vòng thông điệp chuẩn của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if !pre_translate(&msg) {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
            if done() {
                return true;
            }
            if std::time::Instant::now() >= end {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Kho tổng hợp `n` mục (ảnh PNG sinh bằng mã) dưới `dir`: đủ bốn loại, mục thứ 4 có tệp đã mất.
    /// Kho giữ tối đa 50 mục mới nhất.
    fn synthetic_store(dir: &std::path::Path, n: usize) -> std::path::PathBuf {
        use darkshot_core::history::Kind;
        use darkshot_core::history_store::{NewEntry, Store};
        use darkshot_core::image::{encode_png, Image};
        use darkshot_core::naming::LocalTime;
        let root = dir.join("history");
        let pics = dir.join("Pictures");
        std::fs::create_dir_all(&pics).unwrap();
        let store = Store::new(root.clone());
        let sizes = [
            (1920, 1080),
            (1280, 720),
            (800, 1000),
            (2560, 1440),
            (640, 360),
            (300, 200),
        ];
        let mut lost = None;
        for i in 0..n {
            let (w, h) = sizes[i % sizes.len()];
            let mut bgra = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let band = ((x * 6 / w) + i as u32) % 6;
                    let (r, g, b): (u8, u8, u8) = [
                        (0x2F, 0x6F, 0xE4),
                        (0x4A, 0xDE, 0x80),
                        (0xFF, 0x7B, 0x72),
                        (0xE7, 0xE9, 0xEC),
                        (0x33, 0x37, 0x3E),
                        (0xF5, 0xB0, 0x41),
                    ][band as usize];
                    let shade = (y * 60 / h) as u8;
                    bgra.extend_from_slice(&[
                        b.saturating_sub(shade),
                        g.saturating_sub(shade),
                        r.saturating_sub(shade),
                        255,
                    ]);
                }
            }
            let img = Image {
                width: w,
                height: h,
                bgra,
            };
            let kind = [
                Kind::Saved,
                Kind::Uploaded,
                Kind::Copied,
                Kind::UploadFallback,
            ][i % 4];
            let day = 18 - (n - 1 - i) as u8 / 3 % 15;
            let local = LocalTime {
                year: 2026,
                month: 11,
                day,
                hour: (8 + i % 12) as u8,
                minute: (i * 7 % 60) as u8,
                second: (i % 60) as u8,
            };
            let path = matches!(kind, Kind::Saved | Kind::UploadFallback).then(|| {
                let p = pics.join(format!("darkshot_2026-11-{day:02}_{i:03}.png"));
                std::fs::write(&p, encode_png(&img).unwrap()).unwrap();
                p
            });
            let new = NewEntry {
                kind,
                unix_millis: 1_790_000_000_000 + i as u64 * 60_000,
                local,
                path: path.clone(),
                url: (kind == Kind::Uploaded)
                    .then(|| format!("https://img.example.com/shots/darkshot_{i:03}.png")),
                expires: (kind == Kind::Uploaded).then(|| {
                    if i % 8 == 1 {
                        String::from("2026-11-10")
                    } else {
                        String::from("2026-12-18")
                    }
                }),
            };
            store.record(new, &img).unwrap();
            if i == n.saturating_sub(4) {
                lost = path;
            }
        }
        if let Some(p) = lost {
            let _ = std::fs::remove_file(p);
        }
        root
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d = std::env::temp_dir().join(format!("darkshot_hw_{tag}_{}_{t}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Kho tạm 60 mục: mở, chờ nạp xong và ảnh nhỏ về, mô phỏng cuộn/chọn 100 lần, đóng; năm vòng,
    /// số đối tượng GDI không tăng dần.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored --test-threads=1 history_window_with_entries`.
    #[test]
    #[ignore]
    fn history_window_with_entries_has_no_gdi_leak() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            VK_DOWN, VK_END, VK_HOME, VK_NEXT, VK_UP,
        };
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_MOUSEWHEEL, WM_VSCROLL};
        let dir = temp_dir("leak");
        let root = synthetic_store(&dir, 60);
        let parent = test_parent(w!("darkshot-history-test-parent-2"));
        let mut counts = Vec::new();
        for _round in 0..5 {
            show_with_root(parent, root.clone());
            assert!(
                pump_until(10_000, || loaded_count() == Some(50)),
                "store did not load: {:?}",
                loaded_count()
            );
            let list = HWND(list_hwnd() as *mut c_void);
            assert!(!list.is_invalid());
            pump_until(3_000, || pending_thumbs() == 0);
            for i in 0..100u32 {
                let key = [VK_DOWN, VK_DOWN, VK_NEXT, VK_UP, VK_END, VK_HOME][i as usize % 6];
                // SAFETY: thông điệp gửi đồng bộ tới danh sách của cửa sổ Lịch sử trong luồng này.
                unsafe {
                    SendMessageW(
                        list,
                        WM_KEYDOWN,
                        Some(WPARAM(key.0 as usize)),
                        Some(LPARAM(0)),
                    );
                    let delta: i16 = if i % 2 == 0 { -120 } else { 120 };
                    SendMessageW(
                        list,
                        WM_MOUSEWHEEL,
                        Some(WPARAM(((delta as u16 as usize) << 16) as usize)),
                        Some(LPARAM(0)),
                    );
                    SendMessageW(list, WM_VSCROLL, Some(WPARAM(3)), Some(LPARAM(0)));
                }
                pump_until(5, || false);
            }
            assert!(selected_index().is_some(), "keyboard must select a row");
            pump_until(3_000, || pending_thumbs() == 0 && !preview_loading());
            close_if_open();
            pump_until(50, || false);
            counts.push(gdi_count());
        }
        assert!(
            counts[4] <= counts[1] + 2,
            "GDI objects grew across opens: {counts:?}"
        );
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Dựng kho demo tổng hợp trong `%DARKSHOT_DEMO_DIR%\Local\darkshot\history` (bố cục của
    /// `DARKSHOT_PROFILE_DIR`). Không đặt biến thì bỏ qua.
    /// Chạy tay: `set DARKSHOT_DEMO_DIR=...` rồi `cargo test -p darkshot-windows -- --ignored make_demo_store`.
    #[test]
    #[ignore]
    fn make_demo_store() {
        let Some(base) = std::env::var_os("DARKSHOT_DEMO_DIR") else {
            return;
        };
        let base = std::path::PathBuf::from(base);
        let local = base.join("Local").join("darkshot");
        let n: usize = std::env::var("DARKSHOT_DEMO_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(16);
        let root = synthetic_store(&local, n);
        assert!(root.join("index.json").exists());
    }
}
