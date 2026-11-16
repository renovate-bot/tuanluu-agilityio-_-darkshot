//! Biểu tượng khay, menu và thông báo.

use crate::lang;
use shotme_core::actions::{NoticeKind, Notifier};
use shotme_core::i18n::Msg;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ffi::c_void;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetSystemMetrics, LoadIconW,
    LoadImageW, PostMessageW, SetForegroundWindow, TrackPopupMenu, HICON, IDI_APPLICATION,
    IMAGE_ICON, LR_SHARED, MF_STRING, SM_CXSMICON, SM_CYSMICON, TPM_BOTTOMALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_APP, WM_NULL,
};

/// Thông điệp riêng của biểu tượng khay.
pub const WM_TRAY: u32 = WM_APP + 1;
/// Thông báo từ luồng khác: `lparam` là `Box<Notice>` đã chuyển quyền sở hữu.
pub const WM_NOTICE: u32 = WM_APP + 2;

const TRAY_UID: u32 = 1;
const ID_CAPTURE: usize = 1;
const ID_OPTIONS: usize = 2;
const ID_ABOUT: usize = 3;
const ID_HELP: usize = 4;
const ID_QUIT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    Capture,
    Options,
    About,
    Help,
    Quit,
}

/// Nội dung thông báo gửi qua `PostMessageW`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub body: String,
}

pub struct Tray {
    hwnd: HWND,
    icon: HICON,
    added: Cell<bool>,
    /// Thông báo đến lúc biểu tượng chưa có.
    pending: RefCell<VecDeque<Notice>>,
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
        // Dùng LR_SHARED nên không cần DestroyIcon. Nạp đúng cỡ nhỏ của khay (16/20/24 vẽ theo lưới).
        let icon = unsafe {
            let hinst = GetModuleHandleW(None)?;
            let res = PCWSTR(std::ptr::without_provenance::<u16>(1));
            let cx = GetSystemMetrics(SM_CXSMICON);
            let cy = GetSystemMetrics(SM_CYSMICON);
            let small = LoadImageW(Some(hinst.into()), res, IMAGE_ICON, cx, cy, LR_SHARED)
                .map(|h| HICON(h.0));
            match small {
                Ok(i) => i,
                Err(_) => match LoadIconW(Some(hinst.into()), res) {
                    Ok(i) => i,
                    Err(_) => LoadIconW(None, IDI_APPLICATION)?,
                },
            }
        };
        let tray = Tray {
            hwnd,
            icon,
            added: Cell::new(false),
            pending: RefCell::new(VecDeque::new()),
        };
        // Explorer có thể chưa sẵn sàng (tự khởi động cùng Windows): không phải lỗi chết.
        tray.try_add();
        Ok(tray)
    }

    /// Biểu tượng đã nằm trong khay chưa.
    pub fn is_added(&self) -> bool {
        self.added.get()
    }

    /// Thử thêm biểu tượng; trả về `true` nếu đã có trong khay.
    pub fn try_add(&self) -> bool {
        let nid = self.full_data();
        // SAFETY: nid được khởi tạo đầy đủ.
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
        self.set_added(ok)
    }

    /// Ghi nhận trạng thái biểu tượng; vừa có thì hiện thông báo đang chờ (nếu có).
    fn set_added(&self, ok: bool) -> bool {
        self.added.set(ok);
        if ok {
            let next = take_notice_to_show(&mut self.pending.borrow_mut());
            if let Some(n) = next {
                self.show_notice(&n);
            }
        }
        ok
    }

    /// Hiện menu tại con trỏ chuột và trả về mục được chọn.
    pub fn show_menu(&self) -> Option<MenuCommand> {
        // SAFETY: menu được tạo và hủy trong hàm này; hwnd thuộc luồng gọi.
        unsafe {
            let menu = CreatePopupMenu().ok()?;
            let capture = HSTRING::from(lang::tr(Msg::TrayCapture));
            let options = HSTRING::from(lang::tr(Msg::TrayOptions));
            let about = HSTRING::from(lang::tr(Msg::TrayAbout));
            let help = HSTRING::from(lang::tr(Msg::TrayHelp));
            let exit = HSTRING::from(lang::tr(Msg::TrayExit));
            let _ = AppendMenuW(menu, MF_STRING, ID_CAPTURE, &capture);
            let _ = AppendMenuW(menu, MF_STRING, ID_OPTIONS, &options);
            let _ = AppendMenuW(menu, MF_STRING, ID_ABOUT, &about);
            let _ = AppendMenuW(menu, MF_STRING, ID_HELP, &help);
            let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, &exit);
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
                ID_OPTIONS => Some(MenuCommand::Options),
                ID_ABOUT => Some(MenuCommand::About),
                ID_HELP => Some(MenuCommand::Help),
                ID_QUIT => Some(MenuCommand::Quit),
                _ => None,
            }
        }
    }

    /// Thêm lại biểu tượng khi Explorer khởi động lại ("TaskbarCreated").
    /// `NIM_ADD` thất bại (biểu tượng có thể vẫn còn) thì thử `NIM_MODIFY` trước khi coi là chưa có.
    pub fn readd(&self) -> bool {
        let nid = self.full_data();
        // SAFETY: nid được khởi tạo đầy đủ.
        let ok = unsafe {
            Shell_NotifyIconW(NIM_ADD, &nid).as_bool()
                || Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool()
        };
        self.set_added(ok)
    }

    /// Hiện bong bóng thông báo. Chỉ gọi ở luồng chính.
    /// Chưa có biểu tượng thì xếp vào hàng đợi, hiện khi biểu tượng được thêm.
    pub fn show_notice(&self, n: &Notice) {
        if !self.added.get() {
            enqueue_notice(&mut self.pending.borrow_mut(), n.clone());
            return;
        }
        let mut nid = self.base_data();
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = match n.kind {
            NoticeKind::Success | NoticeKind::Info => NIIF_INFO,
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

/// Số thông báo giữ lại tối đa trong lúc biểu tượng khay chưa có.
pub const MAX_QUEUED_NOTICES: usize = 8;

/// Xếp một thông báo vào hàng đợi; đầy thì bỏ cái cũ nhất.
pub fn enqueue_notice(q: &mut VecDeque<Notice>, n: Notice) {
    while q.len() >= MAX_QUEUED_NOTICES {
        q.pop_front();
    }
    q.push_back(n);
}

/// Chọn thông báo để hiện khi biểu tượng vừa có: cái mới nhất mức Error nếu có, không thì cái
/// mới nhất. Hàng đợi được làm rỗng.
pub fn take_notice_to_show(q: &mut VecDeque<Notice>) -> Option<Notice> {
    let pick = match q.iter().rposition(|n| n.kind == NoticeKind::Error) {
        Some(i) => q.remove(i),
        None => q.pop_back(),
    };
    q.clear();
    pick
}

/// Số lần thử lại tối đa khi chưa thêm được biểu tượng.
pub const MAX_ADD_RETRIES: u32 = 30;

/// Có nên thử thêm lại biểu tượng khay không (`attempts` là số lần đã thử lại).
pub fn should_retry(added: bool, attempts: u32) -> bool {
    !added && attempts < MAX_ADD_RETRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phân tích ICONDIR của `res/shotme.ico`: đủ tám cỡ, 32 bit, offset/kích thước hợp lệ.
    #[test]
    fn ico_has_all_sizes_32bit_within_file() {
        let ico: &[u8] = include_bytes!("../res/shotme.ico");
        assert!(ico.len() >= 6, "file quá ngắn");
        let u16at = |o: usize| u16::from_le_bytes([ico[o], ico[o + 1]]) as usize;
        let u32at =
            |o: usize| u32::from_le_bytes([ico[o], ico[o + 1], ico[o + 2], ico[o + 3]]) as usize;
        assert_eq!(u16at(0), 0, "reserved");
        assert_eq!(u16at(2), 1, "type phải là icon");
        let count = u16at(4);
        assert!(ico.len() >= 6 + 16 * count, "thiếu mục ICONDIRENTRY");
        let mut sizes = Vec::new();
        for i in 0..count {
            let e = 6 + 16 * i;
            let w = if ico[e] == 0 { 256 } else { ico[e] as usize };
            let h = if ico[e + 1] == 0 {
                256
            } else {
                ico[e + 1] as usize
            };
            assert_eq!(w, h, "cỡ phải vuông");
            assert_eq!(u16at(e + 6), 32, "cỡ {w} phải 32 bit");
            let size = u32at(e + 8);
            let off = u32at(e + 12);
            assert!(size > 0 && off + size <= ico.len(), "cỡ {w} vượt file");
            sizes.push(w);
        }
        sizes.sort_unstable();
        assert_eq!(sizes, [16, 20, 24, 32, 40, 48, 64, 256]);
    }

    /// Khung phải là bốn góc chữ L rời nhau: điểm giữa cạnh trên và cạnh trái (cỡ 32) là nền xanh.
    #[test]
    fn ico_32_frame_edges_are_open_at_midpoints() {
        let ico: &[u8] = include_bytes!("../res/shotme.ico");
        let u32at =
            |o: usize| u32::from_le_bytes([ico[o], ico[o + 1], ico[o + 2], ico[o + 3]]) as usize;
        let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
        let e = (0..count)
            .map(|i| 6 + 16 * i)
            .find(|&e| ico[e] == 32)
            .expect("thiếu cỡ 32");
        let base = u32at(e + 12) + 40; // bỏ BITMAPINFOHEADER; điểm ảnh BGRA, từ dưới lên
        let red = |x: usize, y: usize| ico[base + ((31 - y) * 32 + x) * 4 + 2];
        let m = (0.285f32 * 32.0).round() as usize; // đường giữa nét của cạnh khung
        assert!(red(16, m) < 128, "giữa cạnh trên bị tô trắng");
        assert!(red(m, 16) < 128, "giữa cạnh trái bị tô trắng");
        assert!(red(m, m) > 200, "góc trên trái phải là nét trắng");
    }

    #[test]
    fn tray_add_retry_policy() {
        assert!(!should_retry(true, 0));
        assert!(should_retry(false, 0));
        assert!(should_retry(false, 29));
        assert!(!should_retry(false, 30));
    }

    fn note(kind: NoticeKind, title: &str) -> Notice {
        Notice {
            kind,
            title: title.into(),
            body: String::new(),
        }
    }

    #[test]
    fn notice_queue_keeps_newest_eight() {
        let mut q = VecDeque::new();
        for i in 0..10 {
            enqueue_notice(&mut q, note(NoticeKind::Info, &i.to_string()));
        }
        assert_eq!(q.len(), MAX_QUEUED_NOTICES);
        let titles: Vec<_> = q.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, ["2", "3", "4", "5", "6", "7", "8", "9"]);
    }

    #[test]
    fn notice_to_show_prefers_newest_error() {
        let mut q = VecDeque::new();
        enqueue_notice(&mut q, note(NoticeKind::Error, "e1"));
        enqueue_notice(&mut q, note(NoticeKind::Error, "e2"));
        enqueue_notice(&mut q, note(NoticeKind::Info, "i3"));
        assert_eq!(
            take_notice_to_show(&mut q),
            Some(note(NoticeKind::Error, "e2"))
        );
        assert!(q.is_empty());

        enqueue_notice(&mut q, note(NoticeKind::Info, "i1"));
        enqueue_notice(&mut q, note(NoticeKind::Info, "i2"));
        assert_eq!(
            take_notice_to_show(&mut q),
            Some(note(NoticeKind::Info, "i2"))
        );
        assert!(q.is_empty());
        assert_eq!(take_notice_to_show(&mut q), None);
    }

    #[test]
    fn fill_wide_truncates_and_terminates() {
        let mut a = [9u16; 4];
        fill_wide(&mut a, "abcdef");
        assert_eq!(a, [97, 98, 99, 0]);
        fill_wide(&mut a, "a");
        assert_eq!(a, [97, 0, 0, 0]);
    }
}
