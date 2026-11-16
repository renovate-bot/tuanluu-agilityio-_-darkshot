//! Phần dùng chung của các cửa sổ phụ (Tùy chọn, Thông tin, Trợ giúp).

use std::ffi::c_void;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, GetMonitorInfoW, MonitorFromPoint, HFONT, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::SystemParametersInfoForDpi;
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, LoadImageW, SendMessageW, SetWindowPos, ICON_BIG, ICON_SMALL,
    IMAGE_ICON, LR_SHARED, NONCLIENTMETRICSW, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON,
    SPI_GETNONCLIENTMETRICS, SWP_NOZORDER, WM_SETICON,
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
