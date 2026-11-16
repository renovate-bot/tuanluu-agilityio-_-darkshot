//! Cửa sổ Cài đặt (Win32 thuần, Common Controls v6 qua manifest).
//!
//! Phần logic (`apply_form`, `capture_hotkey`) là hàm thuần có kiểm thử; phần cửa sổ mỏng.
//! `show` chạy vòng lặp thông điệp riêng (hộp thoại kiểu modal) và bảo đảm khi trả về:
//! Lưu thì phím tắt mới (nếu đổi) đã được đăng ký cho `parent`; Hủy thì phím tắt cũ đã đăng ký lại.
//! Việc ghi `Config` ra đĩa và bật/tắt khởi động cùng Windows do bên gọi (Task 11) làm.

use crate::hotkey;
use shotme_core::config::{normalize_public_base_url, Config, Expiry, Hotkey, R2Config};
use shotme_core::upload::R2Uploader;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DeleteObject, GetMonitorInfoW, GetSysColorBrush, MonitorFromPoint,
    SetBkMode, SetTextColor, COLOR_BTNFACE, HBRUSH, HDC, HFONT, HGDIOBJ, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, TRANSPARENT,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForWindow, SystemParametersInfoForDpi,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, SetFocus, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU,
    VK_RETURN, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, IsChild,
    IsDialogMessageW, IsIconic, KillTimer, LoadImageW, PostMessageW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetForegroundWindow, SetTimer, SetWindowPos, SetWindowTextW,
    ShowWindow, TranslateMessage, BM_CLICK, BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX,
    BS_DEFPUSHBUTTON, BS_GROUPBOX, BS_PUSHBUTTON, CBS_DROPDOWNLIST, CB_ADDSTRING, CB_GETCURSEL,
    CB_GETDROPPEDSTATE, CB_SETCURSEL, CW_USEDEFAULT, ES_AUTOHSCROLL, ES_PASSWORD, ES_READONLY,
    GWL_STYLE, HMENU, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_SHARED, MSG, NONCLIENTMETRICSW,
    SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SPI_GETNONCLIENTMETRICS, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_RESTORE, SW_SHOW, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE,
    WM_APP, WM_CLOSE, WM_COMMAND, WM_CTLCOLORSTATIC, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_NCLBUTTONDOWN, WM_NCRBUTTONDOWN, WM_RBUTTONDOWN, WM_SETFONT, WM_SETICON,
    WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD,
    WS_EX_APPWINDOW, WS_EX_CONTROLPARENT, WS_GROUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

/// Giá trị đọc từ các ô của cửa sổ.
#[derive(Clone)]
pub struct Form {
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub public_base_url: String,
    pub expiry: Expiry,
    pub hotkey: Hotkey,
    pub save_dir: String,
    pub autostart: bool,
}

// Debug tự viết để khóa bí mật không bao giờ lọt vào log hay thông báo lỗi.
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Form")
            .field("account_id", &self.account_id)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("bucket", &self.bucket)
            .field("public_base_url", &self.public_base_url)
            .field("expiry", &self.expiry)
            .field("hotkey", &self.hotkey)
            .field("save_dir", &self.save_dir)
            .field("autostart", &self.autostart)
            .finish()
    }
}

pub struct SettingsResult {
    pub config: Config,
}

fn is_account_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

fn is_bucket_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'
}

/// Gộp biểu mẫu vào cấu hình hiện tại. Thuần: không đụng hệ điều hành.
pub fn apply_form(current: &Config, form: &Form) -> Result<Config, String> {
    let account_id = form.account_id.trim().to_string();
    let access_key_id = form.access_key_id.trim().to_string();
    let bucket = form.bucket.trim().to_string();
    let secret = form.secret_access_key.trim();
    let url = form.public_base_url.trim();
    let save_dir = form.save_dir.trim();

    if !account_id.chars().all(is_account_char) {
        return Err("Account ID chỉ gồm chữ cái, chữ số và dấu gạch ngang.".into());
    }
    if !bucket.chars().all(is_bucket_char) {
        return Err("Tên kho chỉ gồm chữ cái, chữ số, dấu chấm, gạch dưới và gạch ngang.".into());
    }
    let public_base_url = if url.is_empty() {
        String::new()
    } else {
        normalize_public_base_url(url)?
    };
    if save_dir.is_empty() {
        return Err("Hãy chọn thư mục lưu ảnh.".into());
    }
    let secret_access_key = if secret.is_empty() {
        current.r2.secret_access_key.clone()
    } else {
        secret.to_string()
    };
    Ok(Config {
        r2: R2Config {
            account_id,
            access_key_id,
            secret_access_key,
            bucket,
            public_base_url,
        },
        expiry: form.expiry,
        hotkey: form.hotkey,
        save_dir: PathBuf::from(save_dir),
        autostart: form.autostart,
    })
}

/// Kết quả xử lý một lần nhả phím khi đang chọn phím tắt mới.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// Phím bổ trợ đứng một mình: chờ tiếp.
    Ignore,
    /// Phím thường cần kèm Ctrl, Alt hoặc Win (Shift một mình không đủ).
    NeedModifier,
    /// Không nuốt phím: để hệ thống xử lý như thường (Alt+F4 đóng cửa sổ).
    PassThrough,
    Got(Hotkey),
}

/// Sự kiện phím khi đang chọn phím tắt, kèm trạng thái phím bổ trợ lúc xảy ra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEv {
    Down {
        vk: u32,
        ctrl: bool,
        alt: bool,
        shift: bool,
        win: bool,
    },
    Up {
        vk: u32,
        ctrl: bool,
        alt: bool,
        shift: bool,
        win: bool,
    },
}

/// Máy trạng thái của chế độ bắt phím: chốt tổ hợp ở KEYDOWN (đúng phím bổ trợ lúc bấm);
/// KEYUP không có KEYDOWN tương ứng bị bỏ, trừ PrintScreen (không sinh KEYDOWN).
#[derive(Debug, Default)]
pub struct CaptureState {
    downs: Vec<u32>,
}

impl CaptureState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, ev: KeyEv) -> Capture {
        match ev {
            KeyEv::Down {
                vk,
                ctrl,
                alt,
                shift,
                win,
            } => {
                let out = capture_hotkey(vk, ctrl, alt, shift, win);
                if matches!(out, Capture::Got(_) | Capture::NeedModifier)
                    && !self.downs.contains(&vk)
                {
                    self.downs.push(vk);
                }
                out
            }
            KeyEv::Up {
                vk,
                ctrl,
                alt,
                shift,
                win,
            } => {
                if let Some(i) = self.downs.iter().position(|k| *k == vk) {
                    self.downs.swap_remove(i);
                    Capture::Ignore
                } else if vk == 0x2C {
                    // PrintScreen không sinh KEYDOWN: nhận ở KEYUP.
                    capture_hotkey(vk, ctrl, alt, shift, win)
                } else {
                    Capture::Ignore
                }
            }
        }
    }
}

/// Phím cho phép đứng một mình làm phím tắt: PrintScreen, Pause, ScrollLock, F1..F24.
fn allowed_bare(vk: u32) -> bool {
    matches!(vk, 0x2C | 0x13 | 0x91 | 0x70..=0x87)
}

fn is_modifier_vk(vk: u32) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

pub fn capture_hotkey(vk: u32, ctrl: bool, alt: bool, shift: bool, win: bool) -> Capture {
    if is_modifier_vk(vk) {
        return Capture::Ignore;
    }
    // Alt+F4 luôn là lệnh đóng cửa sổ của hệ thống.
    if vk == 0x73 && alt {
        return Capture::PassThrough;
    }
    // Shift một mình không đủ: sẽ chặn việc gõ chữ hoa.
    if ctrl || alt || win || allowed_bare(vk) {
        Capture::Got(Hotkey {
            ctrl,
            alt,
            shift,
            win,
            vk,
        })
    } else {
        Capture::NeedModifier
    }
}

// ---------------------------------------------------------------------------
// Cửa sổ
// ---------------------------------------------------------------------------

const ID_OK: i32 = 1;
const ID_CANCEL: i32 = 2;
const ID_ACCOUNT: i32 = 101;
const ID_KEYID: i32 = 102;
const ID_SECRET: i32 = 103;
const ID_BUCKET: i32 = 104;
const ID_URL: i32 = 105;
const ID_TEST: i32 = 106;
const ID_EXPIRY: i32 = 107;
const ID_DIR: i32 = 108;
const ID_BROWSE: i32 = 109;
const ID_HK_CHANGE: i32 = 110;
const ID_AUTOSTART: i32 = 111;

const fn ws(v: i32) -> WINDOW_STYLE {
    WINDOW_STYLE(v as u32)
}

const TEST_TIMER_ID: usize = 1;
/// Thời gian chờ tối đa của `test_connection` (60 s) cộng dư.
const TEST_FAILSAFE_MS: u32 = 70_000;
const WM_APP_TEST_DONE: u32 = WM_APP + 1;
const VK_CONTROL_U: i32 = VK_CONTROL.0 as i32;

#[derive(Clone, Copy)]
struct Ctl {
    account: HWND,
    keyid: HWND,
    secret: HWND,
    bucket: HWND,
    url: HWND,
    test_btn: HWND,
    test_res: HWND,
    expiry: HWND,
    dir: HWND,
    hk_edit: HWND,
    hk_btn: HWND,
    autostart: HWND,
    err: HWND,
}

struct St {
    current: Config,
    parent: HWND,
    hotkey: Hotkey,
    capturing: bool,
    cap: CaptureState,
    /// Phím tắt cũ đã bị hủy đăng ký (do "Đổi…" hoặc lần Lưu lỗi) và cần đăng ký lại nếu Hủy.
    hk_unregistered: bool,
    done: bool,
    result: Option<Config>,
    font: Option<HFONT>,
}

thread_local! {
    /// HWND của cửa sổ đang mở (0 = không có).
    static TEST_RUNNING: Cell<bool> = const { Cell::new(false) };
    static OPEN: Cell<isize> = const { Cell::new(0) };
    static CTL: Cell<Option<Ctl>> = const { Cell::new(None) };
    static ST: RefCell<Option<St>> = const { RefCell::new(None) };
}

/// Thế hệ cửa sổ: kết quả của luồng nền thuộc cửa sổ đã đóng sẽ bị bỏ.
static GENERATION: AtomicU64 = AtomicU64::new(0);
static TEST_RESULT: Mutex<Option<(u64, String)>> = Mutex::new(None);

fn with_st<R>(f: impl FnOnce(&mut St) -> R) -> Option<R> {
    ST.with(|s| s.borrow_mut().as_mut().map(f))
}

fn ctl() -> Option<Ctl> {
    CTL.with(|c| c.get())
}

fn get_text(h: HWND) -> String {
    // SAFETY: h là cửa sổ con của luồng này; bộ đệm đủ chỗ cho chuỗi cộng ký tự kết thúc.
    unsafe {
        let n = GetWindowTextLengthW(h);
        if n <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; n as usize + 1];
        let got = GetWindowTextW(h, &mut buf).max(0) as usize;
        let s = String::from_utf16_lossy(&buf[..got.min(buf.len())]);
        buf.fill(0);
        s
    }
}

fn set_text(h: HWND, s: &str) {
    // SAFETY: HSTRING sống suốt lời gọi.
    unsafe {
        let _ = SetWindowTextW(h, &HSTRING::from(s));
    }
}

fn read_form() -> Option<Form> {
    let c = ctl()?;
    // SAFETY: các HWND là điều khiển của cửa sổ đang mở ở luồng này.
    let (sel, checked) = unsafe {
        (
            SendMessageW(c.expiry, CB_GETCURSEL, None, None).0,
            SendMessageW(c.autostart, BM_GETCHECK, None, None).0,
        )
    };
    let expiry = usize::try_from(sel)
        .ok()
        .and_then(|i| Expiry::ALL.get(i).copied())
        .unwrap_or_default();
    let hotkey = with_st(|s| s.hotkey)?;
    Some(Form {
        account_id: get_text(c.account),
        access_key_id: get_text(c.keyid),
        secret_access_key: get_text(c.secret),
        bucket: get_text(c.bucket),
        public_base_url: get_text(c.url),
        expiry,
        hotkey,
        save_dir: get_text(c.dir),
        autostart: checked == 1,
    })
}

fn show_error(msg: &str) {
    if let Some(c) = ctl() {
        set_text(c.err, msg);
    }
}

fn refresh_hotkey_text() {
    if let (Some(c), Some(hk)) = (ctl(), with_st(|s| s.hotkey)) {
        set_text(c.hk_edit, &hotkey::describe(&hk));
    }
}

fn finish(result: Option<Config>) {
    with_st(|s| {
        s.result = result;
        s.done = true;
    });
}

fn cancel() {
    with_st(|s| {
        if s.hk_unregistered {
            // Đăng ký lại phím tắt cũ; nếu bị chiếm giữa chừng thì không còn cách nào hơn.
            let _ = hotkey::register(s.parent, &s.current.hotkey);
            s.hk_unregistered = false;
        }
    });
    finish(None);
}

fn save() {
    let Some(form) = read_form() else { return };
    let Some(current) = with_st(|s| s.current.clone()) else {
        return;
    };
    let cfg = match apply_form(&current, &form) {
        Ok(c) => c,
        Err(e) => {
            show_error(&e);
            return;
        }
    };
    let need_rereg =
        with_st(|s| s.hk_unregistered || s.hotkey != s.current.hotkey).unwrap_or(false);
    if need_rereg {
        let parent = with_st(|s| s.parent);
        if let Some(p) = parent {
            hotkey::unregister(p);
            if !hotkey::register(p, &cfg.hotkey) {
                with_st(|s| s.hk_unregistered = true);
                show_error(&format!(
                    "Phím {} đang bị ứng dụng khác chiếm. Hãy chọn phím khác.",
                    hotkey::describe(&cfg.hotkey)
                ));
                return;
            }
            with_st(|s| s.hk_unregistered = false);
        }
    }
    finish(Some(cfg));
}

fn on_command(hwnd: HWND, id: i32) {
    match id {
        ID_OK => save(),
        ID_CANCEL => cancel(),
        ID_TEST => on_test(hwnd),
        ID_BROWSE => on_browse(hwnd),
        ID_HK_CHANGE => on_change_hotkey(),
        _ => {}
    }
}

fn on_change_hotkey() {
    let Some(c) = ctl() else { return };
    let started = with_st(|s| {
        if s.capturing {
            return false;
        }
        hotkey::unregister(s.parent);
        s.hk_unregistered = true;
        s.capturing = true;
        s.cap = CaptureState::new();
        true
    })
    .unwrap_or(false);
    if started {
        set_text(c.hk_edit, "Nhấn tổ hợp phím mới… (Esc để bỏ qua)");
        show_error("");
    }
}

/// Thoát chế độ bắt phím, giữ phím tắt đang chọn trong biểu mẫu.
fn end_capture() {
    let was = with_st(|s| std::mem::replace(&mut s.capturing, false)).unwrap_or(false);
    if was {
        refresh_hotkey_text();
    }
}

fn key_down(k: i32) -> bool {
    // SAFETY: chỉ đọc trạng thái phím.
    unsafe { GetKeyState(k) < 0 }
}

/// Xử lý thông điệp khi đang chọn phím tắt. Trả về `true` nếu đã nuốt thông điệp.
fn capture_msg(msg: &MSG, hk_btn: HWND) -> bool {
    match msg.message {
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_NCLBUTTONDOWN | WM_NCRBUTTONDOWN => {
            // Bấm ra ngoài nút "Đổi…": thoát chế độ bắt rồi xử lý cú bấm bình thường.
            if msg.hwnd != hk_btn {
                end_capture();
            }
            false
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
            let is_down = msg.message == WM_KEYDOWN || msg.message == WM_SYSKEYDOWN;
            let vk = msg.wParam.0 as u32;
            if is_down {
                if vk == VK_ESCAPE.0 as u32 {
                    end_capture();
                    return true;
                }
                // Tự lặp phím (kể cả phím giữ từ trước khi vào chế độ bắt): bỏ qua.
                if (msg.lParam.0 >> 30) & 1 == 1 {
                    return true;
                }
            }
            let (ctrl, alt, shift, win) = (
                key_down(VK_CONTROL_U),
                key_down(VK_MENU.0 as i32),
                key_down(VK_SHIFT.0 as i32),
                key_down(VK_LWIN.0 as i32) || key_down(VK_RWIN.0 as i32),
            );
            let ev = if is_down {
                KeyEv::Down {
                    vk,
                    ctrl,
                    alt,
                    shift,
                    win,
                }
            } else {
                KeyEv::Up {
                    vk,
                    ctrl,
                    alt,
                    shift,
                    win,
                }
            };
            let Some(out) = with_st(|s| s.cap.feed(ev)) else {
                return false;
            };
            match out {
                Capture::Ignore => true,
                Capture::PassThrough => false,
                Capture::NeedModifier => {
                    if let Some(c) = ctl() {
                        set_text(c.hk_edit, "Phím này cần kèm Ctrl, Alt hoặc Win. Thử lại…");
                    }
                    true
                }
                Capture::Got(h) => {
                    with_st(|s| {
                        s.hotkey = h;
                        s.capturing = false;
                    });
                    refresh_hotkey_text();
                    true
                }
            }
        }
        _ => false,
    }
}

fn on_browse(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    // SAFETY: COM được khởi tạo và giải phóng cân bằng trong hàm này; chuỗi do COM cấp được giải phóng bằng CoTaskMemFree.
    let picked: Option<String> = unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let inited = hr.is_ok();
        let r = (|| -> windows::core::Result<String> {
            let dlg: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            let opts = dlg.GetOptions()?;
            dlg.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)?;
            dlg.SetTitle(&HSTRING::from("Chọn thư mục lưu ảnh"))?;
            dlg.Show(Some(hwnd))?;
            let item = dlg.GetResult()?;
            let name: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let s = name.to_string();
            CoTaskMemFree(Some(name.0 as *const c_void));
            Ok(s.unwrap_or_default())
        })();
        if inited {
            CoUninitialize();
        }
        r.ok().filter(|s| !s.is_empty())
    };
    if let Some(p) = picked {
        set_text(c.dir, &p);
    }
}

fn on_test(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    let (Some(form), Some(current)) = (read_form(), with_st(|s| s.current.clone())) else {
        return;
    };
    let cfg = match apply_form(&current, &form) {
        Ok(c) => c,
        Err(e) => {
            show_error(&e);
            return;
        }
    };
    show_error("");
    if !cfg.r2.is_complete() {
        set_text(
            c.test_res,
            "Hãy điền đủ Account ID, Access Key ID, Secret Access Key, Tên kho và Địa chỉ công khai trước.",
        );
        return;
    }
    if TEST_RUNNING.with(|r| r.get()) {
        return;
    }
    // Thế hệ mới: kết quả của lần chạy trước (nếu có) sẽ bị bỏ.
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let target = hwnd.0 as isize;
    let r2 = cfg.r2;
    let spawned = std::thread::Builder::new()
        .name("shotme-test-connection".into())
        .spawn(move || {
            let msg = match R2Uploader::new(&r2).test_connection() {
                Ok(()) => "Thành công: tải lên được và link mở được.".to_string(),
                Err(e) => e.message_vi(),
            };
            if let Ok(mut slot) = TEST_RESULT.lock() {
                // Cửa sổ đã đóng (thế hệ đã đổi): bỏ kết quả, không đè lên kết quả của cửa sổ mới.
                if generation == GENERATION.load(Ordering::SeqCst) {
                    *slot = Some((generation, msg));
                }
            }
            // SAFETY: nếu cửa sổ đã đóng thì PostMessageW trả lỗi và ta bỏ qua; kết quả nằm trong mutex, không có con trỏ treo.
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(target as *mut c_void)),
                    WM_APP_TEST_DONE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        });
    if spawned.is_err() {
        set_text(c.test_res, "Không tạo được luồng kiểm tra. Hãy thử lại.");
        return;
    }
    TEST_RUNNING.with(|r| r.set(true));
    // SAFETY: chuyển focus sang ô còn bật trước khi làm mờ nút; bộ hẹn giờ dự phòng gắn với cửa sổ này.
    unsafe {
        if GetFocus() == c.test_btn {
            let _ = SetFocus(Some(c.account));
        }
        let _ = EnableWindow(c.test_btn, false);
        SetTimer(Some(hwnd), TEST_TIMER_ID, TEST_FAILSAFE_MS, None);
    }
    set_text(c.test_res, "Đang kiểm tra…");
}

/// Bật lại nút "Kiểm tra kết nối" và hủy bộ hẹn giờ dự phòng.
fn test_finished(hwnd: HWND, c: &Ctl) {
    TEST_RUNNING.with(|r| r.set(false));
    // SAFETY: bật lại nút và hủy bộ hẹn giờ của chính cửa sổ này.
    unsafe {
        let _ = KillTimer(Some(hwnd), TEST_TIMER_ID);
        let _ = EnableWindow(c.test_btn, true);
    }
}

fn on_test_timeout(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    if !TEST_RUNNING.with(|r| r.get()) {
        return;
    }
    // Bỏ kết quả muộn của luồng nền.
    GENERATION.fetch_add(1, Ordering::SeqCst);
    set_text(c.test_res, "Kiểm tra quá thời gian chờ. Hãy thử lại.");
    test_finished(hwnd, &c);
}

fn on_test_done(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    let taken = TEST_RESULT.lock().ok().and_then(|mut s| s.take());
    let Some((generation, msg)) = taken else {
        return;
    };
    if generation != GENERATION.load(Ordering::SeqCst) {
        return;
    }
    set_text(c.test_res, &msg);
    test_finished(hwnd, &c);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let code = (wp.0 >> 16) & 0xFFFF;
            if code == 0 {
                on_command(hwnd, (wp.0 & 0xFFFF) as i32);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            cancel();
            LRESULT(0)
        }
        WM_APP_TEST_DONE => {
            on_test_done(hwnd);
            LRESULT(0)
        }
        WM_TIMER if wp.0 == TEST_TIMER_ID => {
            on_test_timeout(hwnd);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            // Mất kích hoạt: thoát chế độ bắt phím (rồi để hệ thống xử lý tiếp, giữ focus đúng).
            if (wp.0 & 0xFFFF) as u32 == WA_INACTIVE {
                end_capture();
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_CTLCOLORSTATIC => {
            if ctl().is_some_and(|c| c.err.0 as isize == lp.0) {
                let hdc = HDC(wp.0 as *mut c_void);
                // SAFETY: hdc do hệ thống cấp cho lần vẽ này; nền lấy từ màu hệ thống.
                unsafe {
                    SetTextColor(hdc, windows::Win32::Foundation::COLORREF(0x0000_00B4));
                    SetBkMode(hdc, TRANSPARENT);
                    let b: HBRUSH = GetSysColorBrush(COLOR_BTNFACE);
                    return LRESULT(b.0 as isize);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

struct Builder {
    parent: HWND,
    dpi: i32,
    all: Vec<HWND>,
}

impl Builder {
    fn s(&self, v: i32) -> i32 {
        (v * self.dpi + 48) / 96
    }

    #[allow(clippy::too_many_arguments)]
    fn make(
        &mut self,
        class: PCWSTR,
        text: &str,
        style: WINDOW_STYLE,
        (x, y, w, h): (i32, i32, i32, i32),
        id: i32,
    ) -> HWND {
        // SAFETY: tạo cửa sổ con của cửa sổ Cài đặt ở luồng chính; lỗi thì trả cửa sổ rỗng (các hàm sau chịu được).
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                &HSTRING::from(text),
                WS_CHILD | WS_VISIBLE | style,
                self.s(x),
                self.s(y),
                self.s(w),
                self.s(h),
                Some(self.parent),
                Some(HMENU(id as usize as *mut c_void)),
                None,
                None,
            )
            .unwrap_or_default()
        };
        self.all.push(hwnd);
        hwnd
    }

    fn label(&mut self, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
        self.make(w!("STATIC"), text, WINDOW_STYLE(0), (x, y, w, h), -1)
    }

    fn edit(&mut self, style: WINDOW_STYLE, rect: (i32, i32, i32, i32), id: i32) -> HWND {
        self.make(
            w!("EDIT"),
            "",
            WS_GROUP | WS_TABSTOP | ws(ES_AUTOHSCROLL) | style | WS_BORDER,
            rect,
            id,
        )
    }

    fn button(
        &mut self,
        text: &str,
        style: WINDOW_STYLE,
        rect: (i32, i32, i32, i32),
        id: i32,
    ) -> HWND {
        self.make(w!("BUTTON"), text, WS_GROUP | WS_TABSTOP | style, rect, id)
    }
}

fn system_font(dpi: u32) -> Option<HFONT> {
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

/// Dựng các điều khiển; trả về chiều cao vùng client (đơn vị DIP) và bộ HWND.
fn build_controls(b: &mut Builder, current: &Config, notice: Option<&str>) -> (i32, Option<Ctl>) {
    const LX: i32 = 24; // lề trái nhãn
    const EX: i32 = 168; // lề trái ô nhập
    const EW: i32 = 282; // bề rộng ô nhập
    let mut y = 12;
    if let Some(n) = notice {
        b.label(n, 12, y, 456, 34);
        y += 42;
    }

    // Nhóm Cloudflare R2
    let g1 = y;
    let g1h = b.make(
        w!("BUTTON"),
        "Cloudflare R2",
        ws(BS_GROUPBOX),
        (12, y, 456, 1),
        -1,
    );
    y += 24;
    let rows = [
        ("Account ID", ID_ACCOUNT, WINDOW_STYLE(0)),
        ("Access Key ID", ID_KEYID, WINDOW_STYLE(0)),
        (
            "Secret Access Key",
            ID_SECRET,
            WINDOW_STYLE(ES_PASSWORD as u32),
        ),
        ("Tên kho (bucket)", ID_BUCKET, WINDOW_STYLE(0)),
    ];
    let mut handles = Vec::new();
    for (label, id, style) in rows {
        b.label(label, LX, y + 3, EX - LX - 4, 20);
        handles.push(b.edit(style, (EX, y, EW, 24), id));
        y += 28;
        if id == ID_SECRET {
            let hint = if current.r2.secret_access_key.is_empty() {
                "Chưa có khóa."
            } else {
                "Đã lưu khóa; để trống nếu không đổi."
            };
            b.label(hint, EX, y - 2, EW, 18);
            y += 20;
        }
    }
    b.label("Địa chỉ công khai", LX, y + 3, EX - LX - 4, 20);
    let url = b.edit(WINDOW_STYLE(0), (EX, y, EW, 24), ID_URL);
    y += 28;
    let test_btn = b.button(
        "Kiểm tra kết nối",
        WS_TABSTOP | ws(BS_PUSHBUTTON),
        (LX, y, 140, 26),
        ID_TEST,
    );
    let test_res = b.label("", EX, y + 4, EW, 36);
    y += 44;
    let g1_h = y - g1;

    // Nhóm Chia sẻ và lưu
    y += 4;
    let g2 = y;
    let g2h = b.make(
        w!("BUTTON"),
        "Chia sẻ và lưu",
        ws(BS_GROUPBOX),
        (12, y, 456, 1),
        -1,
    );
    y += 24;
    b.label("Hạn của link", LX, y + 3, EX - LX - 4, 20);
    let expiry = b.make(
        w!("COMBOBOX"),
        "",
        WS_GROUP | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        (EX, y, 160, 200),
        ID_EXPIRY,
    );
    y += 32;
    b.label("Thư mục lưu", LX, y + 3, EX - LX - 4, 20);
    let dir = b.edit(WINDOW_STYLE(0), (EX, y, EW - 76, 24), ID_DIR);
    b.button(
        "Chọn…",
        ws(BS_PUSHBUTTON),
        (EX + EW - 70, y - 1, 70, 26),
        ID_BROWSE,
    );
    y += 34;
    let g2_h = y - g2;

    // Nhóm Chung
    y += 4;
    let g3 = y;
    let g3h = b.make(w!("BUTTON"), "Chung", ws(BS_GROUPBOX), (12, y, 456, 1), -1);
    y += 24;
    b.label("Phím tắt chụp", LX, y + 3, EX - LX - 4, 20);
    let hk_edit = b.edit(WINDOW_STYLE(ES_READONLY as u32), (EX, y, EW - 76, 24), -1);
    let hk_btn = b.button(
        "Đổi…",
        ws(BS_PUSHBUTTON),
        (EX + EW - 70, y - 1, 70, 26),
        ID_HK_CHANGE,
    );
    y += 32;
    let autostart = b.button(
        "Khởi động cùng Windows",
        ws(BS_AUTOCHECKBOX),
        (LX, y, 300, 22),
        ID_AUTOSTART,
    );
    y += 30;
    let g3_h = y - g3;

    // Dòng lỗi và nút
    y += 8;
    let err = b.label("", 12, y, 456, 36);
    y += 40;
    b.button("Lưu", ws(BS_DEFPUSHBUTTON), (300, y, 80, 28), ID_OK);
    // Hủy không có WS_GROUP: Lưu và Hủy cùng một nhóm mũi tên.
    b.make(
        w!("BUTTON"),
        "Hủy",
        WS_TABSTOP | ws(BS_PUSHBUTTON),
        (388, y, 80, 28),
        ID_CANCEL,
    );
    y += 40;

    // Giờ mới biết chiều cao thật của ba nhóm.
    for (h, top, height) in [(g1h, g1, g1_h), (g2h, g2, g2_h), (g3h, g3, g3_h)] {
        // SAFETY: đổi kích thước hộp nhóm vừa tạo.
        unsafe {
            let _ = SetWindowPos(
                h,
                None,
                b.s(12),
                b.s(top),
                b.s(456),
                b.s(height),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
    let ctl = (|| {
        Some(Ctl {
            account: *handles.first()?,
            keyid: *handles.get(1)?,
            secret: *handles.get(2)?,
            bucket: *handles.get(3)?,
            url,
            test_btn,
            test_res,
            expiry,
            dir,
            hk_edit,
            hk_btn,
            autostart,
            err,
        })
    })();
    (y, ctl)
}

/// Đưa cửa sổ Cài đặt đang mở (nếu có) ra trước; không làm gì khác.
pub fn focus_existing() {
    let open = OPEN.with(|o| o.get());
    if open == 0 {
        return;
    }
    let h = HWND(open as *mut c_void);
    // SAFETY: `h` là cửa sổ Cài đặt của luồng này, OPEN chỉ khác 0 khi cửa sổ còn sống.
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let _ = SetForegroundWindow(h);
    }
}

/// Mở cửa sổ Cài đặt và chờ đến khi người dùng Lưu hoặc Hủy. `None` khi Hủy (hoặc đã có cửa sổ mở).
pub fn show(parent: HWND, current: &Config, notice: Option<&str>) -> Option<SettingsResult> {
    let open = OPEN.with(|o| o.get());
    if open != 0 {
        let h = HWND(open as *mut c_void);
        // SAFETY: đưa cửa sổ Cài đặt đang có ra trước.
        unsafe {
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            let _ = SetForegroundWindow(h);
        }
        return None;
    }

    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; mọi handle được giải phóng ở cuối hàm.
    let hwnd = unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let class = w!("shotme-settings");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..Default::default()
        };
        // Đăng ký lại lần hai báo lỗi vô hại.
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT | WS_EX_APPWINDOW,
            class,
            w!("shotme – Cài đặt"),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            100,
            100,
            Some(parent),
            None,
            Some(hinst.into()),
            None,
        )
        .ok()?;
        // Icon lớn (thanh tác vụ, Alt+Tab) và nhỏ (thanh tiêu đề); LR_SHARED nên không phải hủy.
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
        hwnd
    };
    GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut s) = TEST_RESULT.lock() {
        *s = None;
    }
    // SAFETY: hwnd vừa tạo.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let font = system_font(dpi);
    let mut b = Builder {
        parent: hwnd,
        dpi: dpi as i32,
        all: Vec::new(),
    };
    let (client_h, controls) = build_controls(&mut b, current, notice);
    let Some(c) = controls else {
        // SAFETY: dọn cửa sổ vừa tạo.
        unsafe {
            let _ = DestroyWindow(hwnd);
            if let Some(f) = font {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
        return None;
    };
    if let Some(f) = font {
        for h in &b.all {
            // SAFETY: đặt phông cho điều khiển con; phông sống đến khi đóng cửa sổ.
            unsafe {
                SendMessageW(*h, WM_SETFONT, Some(WPARAM(f.0 as usize)), Some(LPARAM(1)));
            }
        }
    }
    CTL.with(|x| x.set(Some(c)));
    OPEN.with(|o| o.set(hwnd.0 as isize));
    ST.with(|s| {
        *s.borrow_mut() = Some(St {
            current: current.clone(),
            parent,
            hotkey: current.hotkey,
            capturing: false,
            cap: CaptureState::new(),
            hk_unregistered: false,
            done: false,
            result: None,
            font,
        })
    });

    // Điền giá trị ban đầu. Ô Secret luôn để trống.
    set_text(c.account, &current.r2.account_id);
    set_text(c.keyid, &current.r2.access_key_id);
    set_text(c.bucket, &current.r2.bucket);
    set_text(c.url, &current.r2.public_base_url);
    set_text(c.dir, &current.save_dir.to_string_lossy());
    refresh_hotkey_text();
    let labels = ["1 ngày", "7 ngày", "30 ngày", "Không hết hạn"];
    // SAFETY: gửi thông điệp tới các điều khiển của cửa sổ này; HSTRING sống suốt lời gọi.
    unsafe {
        for l in labels {
            let hs = HSTRING::from(l);
            SendMessageW(
                c.expiry,
                CB_ADDSTRING,
                None,
                Some(LPARAM(hs.as_ptr() as isize)),
            );
        }
        let idx = Expiry::ALL
            .iter()
            .position(|e| *e == current.expiry)
            .unwrap_or(2);
        SendMessageW(c.expiry, CB_SETCURSEL, Some(WPARAM(idx)), None);
        SendMessageW(
            c.autostart,
            BM_SETCHECK,
            Some(WPARAM(usize::from(current.autostart))),
            None,
        );
    }

    // Đổi kích thước theo DPI, căn giữa màn hình đang có con trỏ.
    // SAFETY: các lời gọi hệ thống chỉ dùng giá trị cục bộ.
    unsafe {
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: b.s(480),
            bottom: b.s(client_h),
        };
        let _ = AdjustWindowRectExForDpi(
            &mut rc,
            WS_CAPTION | WS_SYSMENU,
            false,
            WS_EX_CONTROLPARENT | WS_EX_APPWINDOW,
            dpi,
        );
        let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
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
                a.left + ((a.right - a.left) - ww) / 2,
                a.top + ((a.bottom - a.top) - wh) / 2,
            )
        } else {
            (100, 100)
        };
        let _ = SetWindowPos(hwnd, None, x, y, ww, wh, SWP_NOZORDER);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(c.account));
    }

    run_loop(hwnd, c);

    // Dọn dẹp: cửa sổ, phông, trạng thái.
    let result = with_st(|s| s.result.take()).flatten();
    // SAFETY: hủy cửa sổ và phông do hàm này tạo.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
    if let Some(Some(f)) = ST.with(|s| s.borrow_mut().take().map(|st| st.font)) {
        // SAFETY: phông do CreateFontIndirectW tạo, cửa sổ dùng nó đã bị hủy.
        unsafe {
            let _ = DeleteObject(HGDIOBJ(f.0));
        }
    }
    CTL.with(|x| x.set(None));
    OPEN.with(|o| o.set(0));
    GENERATION.fetch_add(1, Ordering::SeqCst);
    result.map(|config| SettingsResult { config })
}

fn run_loop(hwnd: HWND, c: Ctl) {
    let mut msg = MSG::default();
    loop {
        if with_st(|s| s.done).unwrap_or(true) {
            break;
        }
        // SAFETY: vòng lặp thông điệp chuẩn của luồng này.
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if r.0 == 0 {
            // WM_QUIT: trả lại cho vòng lặp ngoài rồi thoát.
            // SAFETY: gửi lại yêu cầu thoát.
            unsafe { PostQuitMessage(msg.wParam.0 as i32) };
            cancel();
            break;
        }
        if r.0 < 0 {
            cancel();
            break;
        }
        // SAFETY: IsChild chỉ đọc quan hệ cửa sổ.
        let mine = msg.hwnd == hwnd || unsafe { IsChild(hwnd, msg.hwnd).as_bool() };
        if mine {
            let capturing = with_st(|s| s.capturing).unwrap_or(false);
            if capturing && capture_msg(&msg, c.hk_btn) {
                continue;
            }
            if msg.message == WM_KEYDOWN {
                let vk = msg.wParam.0 as u32;
                // SAFETY: chỉ đọc trạng thái hộp thả.
                let dropped =
                    unsafe { SendMessageW(c.expiry, CB_GETDROPPEDSTATE, None, None).0 != 0 };
                if vk == VK_ESCAPE.0 as u32 && !dropped {
                    cancel();
                    continue;
                }
                if vk == VK_RETURN.0 as u32 && !dropped {
                    // SAFETY: chỉ đọc điều khiển đang giữ focus.
                    let focus = unsafe { GetFocus() };
                    let is_push = unsafe {
                        let st = GetWindowLongPtrW(focus, GWL_STYLE) as u32;
                        let kind = st & 0xF;
                        focus != HWND::default()
                            && (kind == BS_PUSHBUTTON as u32 || kind == BS_DEFPUSHBUTTON as u32)
                            && focus != c.autostart
                            && get_class_is_button(focus)
                    };
                    if is_push {
                        // SAFETY: bấm nút đang giữ focus.
                        unsafe {
                            SendMessageW(focus, BM_CLICK, None, None);
                        }
                    } else {
                        save();
                    }
                    continue;
                }
            }
            // SAFETY: xử lý phím điều hướng (Tab, mũi tên) của hộp thoại.
            if unsafe { IsDialogMessageW(hwnd, &msg).as_bool() } {
                continue;
            }
        }
        // SAFETY: chuyển thông điệp chuẩn.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn get_class_is_button(h: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
    let mut buf = [0u16; 16];
    // SAFETY: bộ đệm cục bộ đủ chỗ.
    let n = unsafe { GetClassNameW(h, &mut buf) }.max(0) as usize;
    String::from_utf16_lossy(&buf[..n.min(buf.len())]).eq_ignore_ascii_case("button")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn base() -> Config {
        let mut c = Config::defaults(Path::new("C:/pics"));
        c.r2 = R2Config {
            account_id: "acc".into(),
            access_key_id: "kid".into(),
            secret_access_key: "OLD".into(),
            bucket: "b".into(),
            public_base_url: "https://img.example.com".into(),
        };
        c
    }

    fn form_of(c: &Config) -> Form {
        Form {
            account_id: c.r2.account_id.clone(),
            access_key_id: c.r2.access_key_id.clone(),
            secret_access_key: String::new(),
            bucket: c.r2.bucket.clone(),
            public_base_url: c.r2.public_base_url.clone(),
            expiry: c.expiry,
            hotkey: c.hotkey,
            save_dir: c.save_dir.to_string_lossy().into_owned(),
            autostart: c.autostart,
        }
    }

    #[test]
    fn empty_secret_keeps_existing() {
        let c = base();
        let mut f = form_of(&c);
        assert_eq!(apply_form(&c, &f).unwrap().r2.secret_access_key, "OLD");
        f.secret_access_key = "NEW".into();
        assert_eq!(apply_form(&c, &f).unwrap().r2.secret_access_key, "NEW");
    }

    #[test]
    fn url_is_normalized_and_bad_url_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.public_base_url = "https://img.example.com/".into();
        assert_eq!(
            apply_form(&c, &f).unwrap().r2.public_base_url,
            "https://img.example.com"
        );
        f.public_base_url = "http://x".into();
        assert!(apply_form(&c, &f).is_err());
    }

    #[test]
    fn empty_url_allowed_when_r2_not_filled() {
        let c = Config::defaults(Path::new("C:/pics"));
        let mut f = form_of(&c);
        f.save_dir = "C:/pics".into();
        let out = apply_form(&c, &f).unwrap();
        assert!(!out.r2.is_complete());
        assert_eq!(out.r2.public_base_url, "");
    }

    #[test]
    fn fields_are_trimmed() {
        let c = base();
        let mut f = form_of(&c);
        f.bucket = " bkt ".into();
        f.account_id = " acc2 ".into();
        f.access_key_id = "\tk2 ".into();
        f.save_dir = "  C:/x  ".into();
        f.secret_access_key = " S ".into();
        let out = apply_form(&c, &f).unwrap();
        assert_eq!(out.r2.bucket, "bkt");
        assert_eq!(out.r2.account_id, "acc2");
        assert_eq!(out.r2.access_key_id, "k2");
        assert_eq!(out.r2.secret_access_key, "S");
        assert_eq!(out.save_dir, PathBuf::from("C:/x"));
    }

    #[test]
    fn empty_save_dir_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.save_dir = "   ".into();
        assert!(apply_form(&c, &f).is_err());
    }

    #[test]
    fn other_fields_are_carried() {
        let c = base();
        let mut f = form_of(&c);
        f.expiry = Expiry::Keep;
        f.autostart = true;
        f.hotkey = Hotkey {
            ctrl: true,
            alt: false,
            shift: false,
            win: false,
            vk: 0x53,
        };
        let out = apply_form(&c, &f).unwrap();
        assert_eq!(out.expiry, Expiry::Keep);
        assert!(out.autostart);
        assert_eq!(out.hotkey, f.hotkey);
    }

    #[test]
    fn bad_account_or_bucket_chars_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.account_id = "a/b".into();
        assert!(apply_form(&c, &f).is_err());
        let mut f = form_of(&c);
        f.bucket = "my bucket".into();
        assert!(apply_form(&c, &f).is_err());
    }

    #[test]
    fn errors_and_debug_never_contain_secret() {
        let c = base();
        let mut f = form_of(&c);
        f.secret_access_key = "SUPER-SECRET".into();
        f.public_base_url = "http://x".into();
        let e = apply_form(&c, &f).unwrap_err();
        assert!(!e.contains("SUPER-SECRET"));
        assert!(!format!("{f:?}").contains("SUPER-SECRET"));
    }

    #[test]
    fn capture_hotkey_rules() {
        // PrintScreen đứng một mình được phép; phím chữ phải kèm phím bổ trợ.
        assert_eq!(
            capture_hotkey(0x2C, false, false, false, false),
            Capture::Got(Hotkey::default())
        );
        assert_eq!(
            capture_hotkey(0x53, false, false, false, false),
            Capture::NeedModifier
        );
        assert_eq!(
            capture_hotkey(0x53, true, false, true, false),
            Capture::Got(Hotkey {
                ctrl: true,
                alt: false,
                shift: true,
                win: false,
                vk: 0x53
            })
        );
        // Chỉ phím bổ trợ: bỏ qua.
        assert_eq!(
            capture_hotkey(0x11, true, false, false, false),
            Capture::Ignore
        );
        assert_eq!(
            capture_hotkey(0x5B, false, false, false, true),
            Capture::Ignore
        );
        // Shift + chữ không đủ.
        assert_eq!(
            capture_hotkey(0x41, false, false, true, false),
            Capture::NeedModifier
        );
        // F-key đứng một mình được phép.
        assert!(matches!(
            capture_hotkey(0x71, false, false, false, false),
            Capture::Got(_)
        ));
    }

    fn dn(vk: u32, ctrl: bool, alt: bool, shift: bool, win: bool) -> KeyEv {
        KeyEv::Down {
            vk,
            ctrl,
            alt,
            shift,
            win,
        }
    }
    fn up(vk: u32, ctrl: bool, alt: bool, shift: bool, win: bool) -> KeyEv {
        KeyEv::Up {
            vk,
            ctrl,
            alt,
            shift,
            win,
        }
    }
    fn hk(ctrl: bool, alt: bool, shift: bool, win: bool, vk: u32) -> Capture {
        Capture::Got(Hotkey {
            ctrl,
            alt,
            shift,
            win,
            vk,
        })
    }

    #[test]
    fn ctrl_released_before_main_key_still_gives_full_combo() {
        let mut st = CaptureState::new();
        // Nhấn Ctrl, Shift (bổ trợ: bỏ qua), rồi S với cả hai đang giữ.
        assert_eq!(
            st.feed(dn(0x11, true, false, false, false)),
            Capture::Ignore
        );
        assert_eq!(st.feed(dn(0x10, true, false, true, false)), Capture::Ignore);
        assert_eq!(
            st.feed(dn(0x53, true, false, true, false)),
            hk(true, false, true, false, 0x53)
        );
        // Ctrl nhả trước S: KEYUP không đổi kết quả.
        assert_eq!(
            st.feed(up(0x11, false, false, true, false)),
            Capture::Ignore
        );
        assert_eq!(
            st.feed(up(0x53, false, false, true, false)),
            Capture::Ignore
        );
    }

    #[test]
    fn orphan_keyup_is_ignored() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(up(0x0D, false, false, false, false)),
            Capture::Ignore
        );
        assert_eq!(
            st.feed(up(0x20, false, false, false, false)),
            Capture::Ignore
        );
    }

    #[test]
    fn printscreen_keyup_only_is_accepted() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(up(0x2C, false, false, false, false)),
            Capture::Got(Hotkey::default())
        );
    }

    #[test]
    fn shift_plus_letter_rejected_but_other_modifiers_accepted() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(dn(0x41, false, false, true, false)),
            Capture::NeedModifier
        );
        assert_eq!(
            st.feed(dn(0x41, true, false, false, false)),
            hk(true, false, false, false, 0x41)
        );
        assert_eq!(
            st.feed(dn(0x41, false, true, false, false)),
            hk(false, true, false, false, 0x41)
        );
        assert_eq!(
            st.feed(dn(0x41, false, false, false, true)),
            hk(false, false, false, true, 0x41)
        );
    }

    #[test]
    fn function_keys_alone_or_with_shift_accepted() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(dn(0x74, false, false, true, false)),
            hk(false, false, true, false, 0x74)
        );
        assert_eq!(
            st.feed(dn(0x74, false, false, false, false)),
            hk(false, false, false, false, 0x74)
        );
    }

    #[test]
    fn alt_f4_is_never_a_combo() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(dn(0x73, false, true, false, false)),
            Capture::PassThrough
        );
        assert_eq!(
            capture_hotkey(0x73, true, true, false, false),
            Capture::PassThrough
        );
    }

    /// Mở cửa sổ thật, đóng bằng WM_CLOSE rồi bằng nút Hủy. Không ghi đĩa, không gọi mạng.
    /// Chạy tay: `cargo test -p shotme-windows -- --ignored settings_window`.
    #[test]
    #[ignore]
    fn settings_window_opens_and_cancels() {
        use windows::Win32::UI::WindowsAndMessaging::{
            DefWindowProcW as Def, FindWindowW, IsWindow, WM_COMMAND,
        };
        unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
            // SAFETY: chuyển nguyên tham số.
            unsafe { Def(h, m, w, l) }
        }
        // Cửa sổ cha ẩn riêng của kiểm thử (phím tắt toàn cục gắn với nó, không đụng ứng dụng thật).
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("shotme-settings-test-parent");
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
        let cfg = Config::defaults(std::path::Path::new("C:/shotme-test-not-created"));
        for use_command in [false, true] {
            let closer = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                // SAFETY: tìm cửa sổ theo tên lớp rồi đăng thông điệp; HWND chỉ dùng như số.
                unsafe {
                    let h = FindWindowW(w!("shotme-settings"), None)
                        .expect("settings window not found");
                    if use_command {
                        PostMessageW(Some(h), WM_COMMAND, WPARAM(ID_CANCEL as usize), LPARAM(0))
                            .unwrap();
                    } else {
                        PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
                    }
                }
            });
            assert!(show(parent, &cfg, Some("Thông báo thử")).is_none());
            closer.join().unwrap();
            // SAFETY: tìm lại cửa sổ sau khi show trả về.
            assert!(unsafe { FindWindowW(w!("shotme-settings"), None) }.is_err());
        }
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            assert!(IsWindow(Some(parent)).as_bool());
            let _ = DestroyWindow(parent);
        }
    }
}
