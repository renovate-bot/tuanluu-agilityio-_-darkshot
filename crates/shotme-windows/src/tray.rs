//! Biểu tượng khay, menu và thông báo.

use shotme_core::actions::{NoticeKind, Notifier};
use std::ffi::c_void;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, PostMessageW,
    SetForegroundWindow, TrackPopupMenu, HICON, IDI_APPLICATION, MF_STRING, TPM_BOTTOMALIGN,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_NULL,
};

/// Thông điệp riêng của biểu tượng khay.
pub const WM_TRAY: u32 = WM_APP + 1;
/// Thông báo từ luồng khác: `lparam` là `Box<Notice>` đã chuyển quyền sở hữu.
pub const WM_NOTICE: u32 = WM_APP + 2;

const TRAY_UID: u32 = 1;
const ID_CAPTURE: usize = 1;
const ID_SETTINGS: usize = 2;
const ID_QUIT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    Capture,
    Settings,
    Quit,
}

/// Nội dung thông báo gửi qua `PostMessageW`.
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub body: String,
}

pub struct Tray {
    hwnd: HWND,
    icon: HICON,
}

fn hwnd_from(v: isize) -> HWND {
    HWND(v as *mut c_void)
}

/// Chép chuỗi vào mảng UTF-16 cố định, cắt bớt và luôn kết thúc bằng 0.
pub fn fill_wide<const N: usize>(dst: &mut [u16; N], s: &str) {
    *dst = [0; N];
    for (slot, u) in dst.iter_mut().take(N - 1).zip(s.encode_utf16()) {
        *slot = u;
    }
}

impl Tray {
    fn base_data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_UID,
            ..Default::default()
        }
    }

    fn full_data(&self) -> NOTIFYICONDATAW {
        let mut nid = self.base_data();
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = self.icon;
        fill_wide(&mut nid.szTip, "shotme");
        nid
    }

    pub fn new(hwnd: HWND) -> windows::core::Result<Tray> {
        // SAFETY: tài nguyên biểu tượng số 1 được nhúng bởi build.rs; thiếu thì dùng biểu tượng mặc định.
        let icon = unsafe {
            let hinst = GetModuleHandleW(None)?;
            match LoadIconW(
                Some(hinst.into()),
                PCWSTR(std::ptr::without_provenance::<u16>(1)),
            ) {
                Ok(i) => i,
                Err(_) => LoadIconW(None, IDI_APPLICATION)?,
            }
        };
        let tray = Tray { hwnd, icon };
        let nid = tray.full_data();
        // SAFETY: nid được khởi tạo đầy đủ ở trên.
        unsafe { Shell_NotifyIconW(NIM_ADD, &nid).ok()? };
        Ok(tray)
    }

    /// Hiện menu tại con trỏ chuột và trả về mục được chọn.
    pub fn show_menu(&self) -> Option<MenuCommand> {
        // SAFETY: menu được tạo và hủy trong hàm này; hwnd thuộc luồng gọi.
        unsafe {
            let menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(menu, MF_STRING, ID_CAPTURE, w!("Chụp màn hình"));
            let _ = AppendMenuW(menu, MF_STRING, ID_SETTINGS, w!("Cài đặt"));
            let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Thoát"));
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            // Cần để menu đóng khi bấm ra ngoài.
            let _ = SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                pt.x,
                pt.y,
                Some(0),
                self.hwnd,
                None,
            );
            let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(menu);
            match cmd.0 as usize {
                ID_CAPTURE => Some(MenuCommand::Capture),
                ID_SETTINGS => Some(MenuCommand::Settings),
                ID_QUIT => Some(MenuCommand::Quit),
                _ => None,
            }
        }
    }

    /// Thêm lại biểu tượng khi Explorer khởi động lại ("TaskbarCreated").
    pub fn readd(&self) {
        let nid = self.full_data();
        // SAFETY: nid được khởi tạo đầy đủ; lỗi được bỏ qua (thử lại ở lần sau).
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }
    }

    /// Hiện bong bóng thông báo. Chỉ gọi ở luồng chính.
    pub fn show_notice(&self, n: &Notice) {
        let mut nid = self.base_data();
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = match n.kind {
            NoticeKind::Info => NIIF_INFO,
            NoticeKind::Error => NIIF_ERROR,
        };
        fill_wide(&mut nid.szInfoTitle, &n.title);
        fill_wide(&mut nid.szInfo, &n.body);
        // SAFETY: nid được khởi tạo đầy đủ.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    pub fn notifier(&self) -> TrayNotifier {
        TrayNotifier {
            hwnd: self.hwnd.0 as isize,
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        let nid = self.base_data();
        // SAFETY: gỡ biểu tượng của chính ta.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }
}

/// Gửi thông báo về luồng chính; an toàn khi gọi từ luồng bất kỳ.
#[derive(Clone)]
pub struct TrayNotifier {
    hwnd: isize,
}

impl Notifier for TrayNotifier {
    fn notify(&self, kind: NoticeKind, title: &str, body: &str) {
        let raw = Box::into_raw(Box::new(Notice {
            kind,
            title: title.to_owned(),
            body: body.to_owned(),
        }));
        // SAFETY: con trỏ chỉ được giải phóng đúng một lần: ở đây nếu gửi thất bại,
        // hoặc ở luồng chính khi xử lý WM_NOTICE.
        let posted = unsafe {
            PostMessageW(
                Some(hwnd_from(self.hwnd)),
                WM_NOTICE,
                WPARAM(0),
                LPARAM(raw as isize),
            )
        };
        if posted.is_err() {
            // SAFETY: tin nhắn không vào hàng đợi nên raw vẫn thuộc về ta.
            drop(unsafe { Box::from_raw(raw) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_wide_truncates_and_terminates() {
        let mut a = [9u16; 4];
        fill_wide(&mut a, "abcdef");
        assert_eq!(a, [97, 98, 99, 0]);
        fill_wide(&mut a, "a");
        assert_eq!(a, [97, 0, 0, 0]);
    }
}
